# Contrato IPC entre Tauri y sensor-agent

## Transporte

- Un objeto JSON por línea UTF-8 (`NDJSON`).
- `stdin`: comandos Rust → sidecar.
- `stdout`: respuestas/eventos sidecar → Rust.
- `stderr`: logs humanos/estructurados, nunca mensajes de protocolo.
- Tamaño máximo inicial por mensaje: 1 MiB.
- Saltos de línea dentro de strings deben ir escapados.

## Envolvente

```json
{
  "protocol_version": 1,
  "session_nonce": "base64url-opaque",
  "sequence": 42,
  "timestamp_utc": "2026-09-17T10:30:00.123Z",
  "type": "sample",
  "payload": {}
}
```

### Reglas

- El padre genera `session_nonce` y lo entrega al sidecar mediante un canal de lanzamiento no registrado.
- `sequence` aumenta por emisor; duplicados y retrocesos se registran y descartan.
- Una versión mayor desconocida impide continuar; una menor compatible puede negociarse.
- Todos los mensajes se validan antes de modificar estado.
- Tras tres mensajes inválidos consecutivos, Rust cierra y reinicia el sidecar con backoff limitado.

## Handshake

### Comando `hello`

```json
{
  "type": "hello",
  "payload": {
    "app_version": "0.1.0",
    "supported_protocols": [1]
  }
}
```

### Evento `hello_ack`

```json
{
  "type": "hello_ack",
  "payload": {
    "agent_version": "0.1.0",
    "selected_protocol": 1,
    "runtime": ".NET",
    "low_level_access": {
      "state": "available",
      "provider": "pawnio",
      "details_code": null
    }
  }
}
```

Estados: `available`, `reduced`, `missing`, `denied`, `error`, `unknown`. Rust los traduce al enum de UI (`not_needed | available | installable | denied | error`) según `data-model.md`; el sidecar nunca decide si el acceso «hace falta».

## Descubrimiento

### Evento `capabilities`

Incluye CPU, grupos y descriptores de sensores. Los IDs originales se tratan como opacos.

```json
{
  "type": "capabilities",
  "payload": {
    "cpu": {
      "vendor": "intel",
      "display_name": "Intel Core ...",
      "logical_processors": 22,
      "physical_cores": 16,
      "hybrid": true
    },
    "groups": [
      {"id": "p", "kind": "p", "logical_count": 12},
      {"id": "e", "kind": "e", "logical_count": 8},
      {"id": "lp-e", "kind": "lp_e", "logical_count": 2}
    ],
    "sensors": [
      {
        "id": "cpu.package.temp",
        "source_id": "opaque-source-id",
        "source_name": "CPU Package",
        "metric": "temperature",
        "scope": "package",
        "scope_ref": null,
        "unit": "celsius",
        "quality": "direct",
        "metadata": {"tjmax_c": 110.0}
      }
    ]
  }
}
```

## Muestreo

Los grupos `p`, `e`, `lp_e` los clasifica el sidecar con `GetLogicalProcessorInformationEx` y CPUID (hoja 0x1A en Intel); si no puede, emite `kind: "unknown"` y Rust trata la CPU como homogénea con aviso de cobertura.

Sensores de bandera (`thermal_flag`, `power_flag`, `current_flag`): si `LibreHardwareMonitorLib` no los expone, el sidecar intenta leerlos del MSR de estado térmico/eléctrico (`IA32_THERM_STATUS`, `IA32_PACKAGE_THERM_STATUS`, `MSR_CORE_PERF_LIMIT_REASONS` en Intel; equivalentes SMU en AMD) a través del acceso de bajo nivel disponible, con `quality: "direct"`. Si no hay acceso, los descriptores no se emiten y Rust limita la confianza máxima a `thermal_probable`.

El sidecar **no** emite reloj efectivo derivado: Rust lo calcula con el contador `\Processor Information(*)\% Processor Performance` y lo registra como descriptor propio de origen `host` con `quality: "derived"`.

`capabilities` se reemite tras un `start` con un `detail` distinto al anterior, tras una reanudación del sistema y tras cada reinicio del proceso. Rust compara el catálogo nuevo con el vigente por `source_id` y marca como `unsupported` los sensores que desaparecen.

### Comando `start`

```json
{
  "type": "start",
  "payload": {
    "interval_ms": 1000,
    "detail": "representative"
  }
}
```

`detail`: `representative`, `per_group`, `per_core`. `interval_ms`: 250–10 000.

### Evento `started`

```json
{ "type": "started", "payload": { "interval_ms": 1000, "detail": "representative" } }
```

Confirma los parámetros efectivos (el sidecar puede ajustar `interval_ms` al mínimo soportado por el hardware y lo indica aquí).

### Evento `sample`

```json
{
  "type": "sample",
  "payload": {
    "monotonic_ms": 12345,
    "duration_ms": 28,
    "values": [
      {"sensor_id": "cpu.package.temp", "number": 96.0, "status": "ok"},
      {"sensor_id": "cpu.thermal.flag", "boolean": true, "status": "ok"},
      {"sensor_id": "cpu.package.power", "number": 31.8, "status": "ok"}
    ]
  }
}
```

Cada valor contiene exactamente uno de `number` o `boolean` cuando `status=ok`; con otro estado puede no contener valor. Estados: `ok`, `missing`, `stale`, `invalid`, `unsupported`.

## Otros comandos

- `set_rate`: cambia `interval_ms` dentro de 250–10 000 ms; responde con `started` con el valor efectivo.
- `snapshot`: solicita una muestra fuera de ciclo sin reiniciar el temporizador.
- `stop`: detiene muestreo y confirma con `stopped`.
- `shutdown`: cierre ordenado; el padre puede terminar el proceso tras timeout.

No existe comando para leer archivos, ejecutar procesos, modificar MSR, controlar ventiladores o cambiar potencia.

## Errores

```json
{
  "type": "error",
  "payload": {
    "code": "SENSOR_ENUMERATION_FAILED",
    "severity": "recoverable",
    "message_key": "collector.sensor_enumeration_failed",
    "context": {"hardware_type": "cpu"}
  }
}
```

- `message_key`, no prosa localizada, es la base de UI.
- El sidecar nunca localiza texto; las claves y enums del protocolo permanecen en inglés.
- `context` solo admite claves documentadas y no incluye excepciones completas en modo normal.
- Severidades: `info`, `recoverable`, `fatal`.

## Ciclo de vida y detección de fallos

- El sidecar termina por sí mismo al recibir EOF en `stdin` o cuando el proceso padre (PID recibido en el lanzamiento) deja de existir; lo comprueba cada 2 s. No existe `ping`/`pong`: un sidecar vivo pero bloqueado se detecta por ausencia de `sample` durante 3 intervalos, tras lo cual Rust lo termina y aplica la política de reinicio.
- Rust detecta la caída del sidecar por EOF en `stdout` o por código de salida.

## Recuperación

1. Ante EOF inesperado, Rust marca datos como obsoletos y conserva la UI.
2. Reinicia como máximo tres veces con backoff en una ventana de diez minutos.
3. Tras agotar intentos, requiere acción del usuario y ofrece resumen técnico.
4. Un reinicio genera catálogo nuevo; no se reutilizan descriptores sin comparar.

## Compatibilidad

- Añadir campos opcionales es compatible.
- Eliminar/renombrar campos o cambiar semántica requiere versión mayor.
- Los enums desconocidos se conservan como `unknown` cuando el esquema lo permita.
- Fixtures canónicos deben serializarse en C# y deserializarse en Rust en CI, y viceversa.

## Límite del contrato

Este protocolo solo comunica Rust con `sensor-agent`. Onboarding, preferencias, ventana, inicio con Windows y actualizaciones pertenecen al contrato de comandos de aplicación descrito en `application-commands.md` y nunca amplían la lista de órdenes privilegiadas del sidecar.
