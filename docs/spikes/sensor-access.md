# Spike T019/T019a: acceso a sensores y nivel de cobertura

- Estado: cerrado para T019a (2026-09-19); quedan abiertas las matrices y spikes posteriores de L03
- Fecha de inicio: 2026-09-19
- Proveedor previsto: PawnIO mediante \`LibreHardwareMonitorLib\` 0.9.6
- Ejecutable de prueba: \`apps/sensor-agent/bin/Debug/net10.0-windows/SensorAgent.exe\`

## Objetivo

Comprobar si el sidecar puede leer los registros necesarios para el nivel A y si el
proveedor permite limpiar los bits de registro sin UAC recurrente. La prueba no instala,
repara ni elimina controladores por sí sola.

## Preparación local

Desde la raíz del repositorio:

\`\`\`powershell
dotnet build .\apps\sensor-agent\SensorAgent.sln --configuration Debug --no-restore
\`\`\`

El probe se ejecuta en un modo separado del protocolo IPC normal:

\`\`\`powershell
& .\apps\sensor-agent\bin\Debug\net10.0-windows\SensorAgent.exe --probe-low-level
\`\`\`

Guardar la salida JSON en un fichero local fuera del repositorio si contiene valores de
registros de una máquina real. No registrar números de serie, nombres de usuario ni rutas.

## Qué comprueba el probe

### Intel

- PawnIO instalado y versión detectada.
- Lectura de \`0x64F\` (\`MSR_CORE_PERF_LIMIT_REASONS\`).
- Lectura de \`0x1A2\` (\`MSR_TEMPERATURE_TARGET\`) y TjMax plausible.
- Lectura de \`0x610\` (\`MSR_PKG_POWER_LIMIT\`).
- Limitación explícita de la versión actual: la API pública de LHM 0.9.6 no ofrece escritura
  de MSR, por lo que \`log_clear_supported\` aparece como \`false\` y no se declara nivel A pleno.

### AMD

- PawnIO instalado y versión detectada.
- Versión del SMU y resolución de la tabla PM.
- La lectura de la tabla y sus offsets todavía debe contrastarse con la familia concreta antes
  de añadir una versión a la lista permitida del normalizador.

## Matriz de ejecución

| Equipo | CPU/familia | Windows/build | Alimentación | PawnIO antes | Lectura sin proveedor | PawnIO instalado una vez | Tras reinicio sin UAC | Resultado A/B/C | Evidencia |
|---|---|---|---|---|---|---|---|---|---|
| Desarrollo | AMD Ryzen 5 2600X / Zen+ (Pinnacle Ridge, Family 17h Model 8) | Windows 11 Pro / 26200 | CA (sin batería detectada) | no instalado | \`missing\`, sin acceso avanzado | \`available\` (elevado): SMU \`0x002B1800\`; tabla PM no fiable | \`denied\` (usuario estándar): SMU \`0x00000000\`, \`SMU_VERSION_ZERO\` → resultado **(b)** | nivel A solo elevado; sin limpieza de bits | probes 2026-09-19 |
| Intel híbrido | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |
| Intel anterior | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |
| Portátil OEM | pendiente | pendiente | batería/CA | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |
| AMD Zen 4 | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |

## Criterios de observación

1. La lectura se ejecuta primero sin elevar y sin proveedor instalado.
2. La instalación del proveedor, si se prueba, requiere una acción UAC separada y consciente.
3. Después de reiniciar, el sidecar se ejecuta como usuario estándar y no solicita UAC.
4. Una lectura de registro no se considera válida solo porque devuelva cero: en Intel \`0x1A2\`
   debe contener un TjMax plausible.
5. La limpieza de \`0x64F\` se considera no demostrada mientras el proveedor no permita una
   operación de escritura restringida a ese registro y no exista una prueba de lectura,
   limpieza y lectura posterior.

## Resultado y decisión

Se completará tras la ejecución real:

- \`(a)\`: nivel A sin UAC recurrente; continuar plan completo.
- \`(b)\`: nivel A requiere un servicio privilegiado; abrir revisión de amenazas antes de seguir.
- \`(c)\`: nivel A no alcanzable; continuar solo como explicador prudente B/C y retirar las
  afirmaciones de clasificación confirmada y potencial cuantificado.

La decisión debe registrarse en un ADR antes de marcar T019a como terminada.

## Ejecuciones registradas

### 2026-09-19 — equipo de desarrollo, sin PawnIO

Comando:

\`\`\`powershell
.\apps\sensor-agent\bin\Debug\net10.0-windows\SensorAgent.exe --probe-low-level
\`\`\`

Resultado relevante:

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "missing",
  "provider": "pawnio",
  "provider_version": null,
  "log_clear_supported": false,
  "details_code": "PAWNIO_NOT_INSTALLED"
}
\`\`\`

No se elevó el proceso, no se instaló ningún controlador y no se modificó ningún registro.

### 2026-09-19 — equipo de desarrollo, PawnIO 2.2.0.0 instalado (proceso elevado)

PawnIO se instaló con su instalador oficial en una acción UAC separada. Estado comprobado
antes del probe: servicio \`PawnIO\` en \`RUNNING\`, dispositivo \`ROOT\PAWNIO\0000\` con
\`Status: OK\`, \`DriverVersion 2.2.0.0\`, \`IsSigned: True\`. No se reinició el equipo.

Primera ejecución (código anterior a la corrección de este día):

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "denied",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "smu_version": null,
  "details_code": "AMD_SMU_READ_FAILED"
}
\`\`\`

Causa raíz (reproducida con un programa aparte contra LibreHardwareMonitorLib 0.9.6): la
sonda detectaba el fabricante abriendo y cerrando \`Computer { IsCpuEnabled = true }\`.
\`Computer.Close()\` llama a \`Mutexes.Close()\`, que cierra el handle de \`Global\Access_PCI\`
sin poner el campo a \`null\`; la siguiente llamada de \`RyzenSmu.GetSmuVersion()\` espera ese
mutex cerrado y LHM lanza \`TimeoutException: Timeout waiting for PCI bus mutex\`. El
\`catch\` genérico lo convertía en \`denied\`. En un proceso sin \`Computer\` la misma llamada
funciona. No tiene relación con firmas de módulos, privilegios ni reinicios pendientes.

Corrección aplicada: el fabricante se lee con CPUID (leaf 0) sin tocar LHM, y el informe
conserva etapa, tipo, mensaje y HResult de la excepción en \`error\`. Regla derivada para el
colector: no mezclar \`Computer.Open()/Close()\` con \`RyzenSmu\`/\`IntelMsr\` en el mismo
proceso.

Segunda ejecución (código corregido, mismo proceso elevado):

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "available",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [],
  "smu_version": "0x002B1800",
  "log_clear_supported": false,
  "details_code": "AMD_PM_TABLE_REQUIRES_ALLOWLIST",
  "error": null
}
\`\`\`

Observaciones:

- \`GetCodeName()\` devuelve 9 (Pinnacle Ridge). \`ResolvePmTable\` no lanza, pero devuelve
  \`version 0x002B0000\` y un tamaño absurdo (\`3144159232\`): la tabla PM de Zen+ no es
  utilizable con LHM 0.9.6 y no debe entrar en la lista permitida.
- Pendiente para cerrar la fila: ejecutar como usuario estándar tras reiniciar (criterio 3)
  y comprobar si el acceso al dispositivo requiere elevación en cada arranque.
- Decisión de producto tomada el mismo día con \`/speckit-clarify\` (\`spec.md\`, FR-087 a
  FR-090): ThrottleWatch empaqueta y ejecuta el instalador oficial de PawnIO; el nivel A se
  obtiene con un lanzador elevado exclusivo del sidecar registrado en ese paso (ADR con
  revisión de amenazas pendiente, T019a); se reutiliza un PawnIO ya instalado y nunca se
  desinstala; si el acceso falla en sesión se degrada a B/C sin interrumpir.

### 2026-09-19 — equipo de desarrollo, usuario estándar tras reiniciar (T152)

Reinicio completo del equipo; PowerShell **sin** elevar (`IsInRole(Administrator)` → `False`);
compilación limpia y ejecución de la sonda corregida:

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "denied",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [],
  "smu_version": "0x00000000",
  "log_clear_supported": false,
  "details_code": "SMU_VERSION_ZERO",
  "error": null
}
\`\`\`

Lectura: el dispositivo PawnIO se abre y el módulo carga sin excepción como usuario estándar,
pero la comunicación con el SMU devuelve cero. Comparado con la ejecución elevada del mismo día
(\`0x002B1800\`), el nivel A solo es alcanzable con el sidecar elevado. Queda confirmado el
resultado **(b)** de \`plan.md\` § Fase 0: continuar solo con un lanzador elevado exclusivo del
sidecar que supere la revisión de amenazas (ADR-0004, pendiente de aprobación por el
propietario del proyecto).

### Cierre T019a — 2026-09-19

T152 confirmó el resultado **(b)**: el SMU devuelve `0x00000000` como usuario estándar tras
reiniciar y `0x002B1800` en el proceso elevado. ADR-0004 queda aceptado con condiciones C1–C6;
por tanto T019a queda cerrada con resultado parcial y la implementación continúa por T153 → T154
→ T155, manteniendo B/C para cuentas estándar. El resto de la matriz de hardware y los demás
spikes de L03 permanecen abiertos.
