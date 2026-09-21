# Modelo de amenazas y revisión de fronteras

Revisión: 2026-09-20 · alcance: compilación de escritorio Windows, sidecar .NET y
actualizador. Este documento complementa ADR-0004; no sustituye la constitución ni los ADR.

## Fronteras de confianza

| Zona | Puede confiar | No debe confiar |
|---|---|---|
| WebView Svelte | contratos Zod, respuestas validadas y estado local | rutas, URLs, procesos, permisos o valores devueltos por el entorno |
| Backend Rust/Tauri | comandos registrados, almacenamiento local y reglaset versionado | argumentos sin validar, rutas recibidas y contenido de la WebView |
| Sidecar .NET | ruta empaquetada, protocolo NDJSON y `Computer` único por proceso | mensajes sin secuencia/nonce y sensores con calidad no demostrada |
| Lanzador elevado | lista cerrada de comandos, nonce efímero y manifiesto firmado | identidad del proceso del mismo usuario: cualquier proceso de ese usuario puede intentarlo |
| Actualizador | endpoint fijo, manifiesto y firma Ed25519 | endpoint configurable, parciales y artefactos no verificados |

La frontera real del paso elevado es el protocolo tipado y su validación, no la identidad del
proceso que abre la tubería. ADR-0004 fija además nombre aleatorio por sesión,
`FILE_FLAG_FIRST_PIPE_INSTANCE`, nonce fuera de la línea de comandos y ausencia de escrituras
genéricas o de MSR. El futuro `ClearLimitLog` sigue reservado y no forma parte de la superficie
actual.

## Amenazas y controles

| Amenaza | Control implementado o exigido | Residuo aceptado |
|---|---|---|
| Cualquier proceso del mismo usuario intenta lanzar el sidecar elevado | Enum cerrado de comandos, nonce por sesión, secuencia, tamaño máximo, ruta fija y manifiesto minisign; Authenticode cuando exista | El proceso puede provocar intentos y denegaciones de disponibilidad durante esa sesión |
| Ocupación del nombre de la tubería | Nombre aleatorio y `FILE_FLAG_FIRST_PIPE_INSTANCE`; se rechaza una instancia existente | Denegación acotada a la sesión, sin reutilizar una tubería ajena |
| Binario sustituido por un actualizador legítimo o malicioso | Manifiesto firmado por la clave de release; clave de desarrollo solo en desarrollo; no se desactiva la verificación | Custodia de la clave de release fuera de este repositorio |
| Escritura arbitraria de MSR o dispositivo | No hay comando de escritura; cualquier futura incorporación debe ser `ClearLimitLog` y tener pruebas negativas | No se puede ofrecer esa funcionalidad hasta aprobarla expresamente |
| Replay o cruce de sesiones | Nonce, `session_id`, secuencia monotónica y EOF/heartbeat como reglas de vida | Un atacante puede consumir CPU intentando mensajes, pero no inyectar una sesión válida sin el nonce |
| Exfiltración desde la WebView | CSP sin orígenes remotos, sin permisos de procesos/opener genérico y comandos Rust con lista cerrada | Vulnerabilidades futuras de WebView2 dependen de la actualización del runtime |
| Logs con identificadores o datos de sensores | Redacción en el envoltorio de registro, exportación anónima y validación en la frontera | El modo detallado es opt-in y expira al reiniciar o a las 24 horas |

## Superficie de comandos revisada

La lista registrada en `apps/desktop/src-tauri/src/lib.rs` es la única superficie Tauri de la
ventana principal:

`get_live_snapshot`, `get_coverage`, `recheck_coverage`, `get_onboarding_state`,
`set_onboarding_state`, `get_window_state`, `set_window_state`, `request_low_level_access`,
`disable_advanced_access`, `get_guided_preflight`, `start_guided`, `stop_guided`,
`get_analysis_window`, `preview_export`, `export`, `cancel_export`, `import_session`,
`get_preferences`, `get_storage_usage`, `get_technical_summary`, `get_third_party_notices`,
`log_frontend`, `delete_monitoring_data`, `reset_application`,
`open_logs_folder`, `open_external_url`, `set_preference`, `get_cpu_topology`, `set_tray_paused`,
`list_sessions`, `get_session`, `get_report`, `delete_session` y `set_session_reference`.

Los comandos reciben DTOs concretos y el puente aplica esquemas Zod antes de invocar. Las
operaciones destructivas requieren su flujo de confirmación en la interfaz; los identificadores
de documentos, URLs y rutas se resuelven contra listas cerradas en Rust. No se concede a la
WebView permiso de proceso ni un permiso de plugin para ejecutar comandos arbitrarios.

## CSP y comprobación de cambios

La configuración de `apps/desktop/src-tauri/tauri.conf.json` fija:

```text
default-src 'self'; style-src 'self' 'unsafe-inline'
```

No hay `connect-src` remoto, `script-src` remoto, `object-src`, `frame-src` ni `worker-src`
adicionales. Si una dependencia exige ampliar la CSP, el cambio debe actualizar este documento,
el spike de permisos y su revisión de amenazas antes de entrar en la compilación de release.

Comprobaciones de esta revisión:

- `node scripts/check-traceability.mjs` — trazabilidad consistente.
- `node scripts/design-sync.mjs --check` — copia del diseño sincronizada.
- `cargo deny check advisories licenses sources` — dependencias Rust dentro de la política,
  incluida la licencia ISC de `libloading` transitiva de `tray-icon`.
- `rg -n "tauri::command|generate_handler" apps/desktop/src-tauri/src` — inventario revisado
  contra `lib.rs`.
