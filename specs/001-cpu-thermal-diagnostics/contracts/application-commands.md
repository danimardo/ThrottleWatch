# Contrato de comandos de aplicación

Frontera tipada entre la interfaz Svelte y el backend Tauri/Rust. No forma parte del protocolo privilegiado de `sensor-agent`. Los nombres, enums y códigos son estables en inglés; la UI localiza `message_key` y nunca recibe prosa como contrato.

## Preferencias

- `get_preferences() -> PreferencesSnapshot`
- `set_preference({ key, value, expected_schema_version }) -> PreferencesSnapshot`
- `reset_preferences() -> void`: solo como parte del restablecimiento total confirmado.

El backend valida claves, tipos y dependencias. En particular, `startup.mode=tray` se rechaza si `tray.monitoring_enabled=false`; desactivar bandeja cambia un inicio oculto incompatible a `window` y una acción de cierre `tray` a `exit` de forma atómica y lo comunica en la respuesta (`PreferencesSnapshot.adjusted[]`). `guided.notify_on_finish=true` se rechaza si `notifications.enabled=false`.

## Onboarding

- `get_onboarding_state() -> OnboardingState`
- `set_onboarding_state({ flow_version, last_slide, status, completed_at, last_seen_notice_version }) -> OnboardingState`
- `set_onboarding_slide({ flow_version, slide: 1..5 }) -> void`
- `resolve_onboarding({ flow_version, outcome: "completed" | "skipped" }) -> void`
- `acknowledge_whats_new({ notice_version }) -> void`

Repetir el recorrido desde Ayuda es un estado de UI y no revierte `completed`/`skipped`.

## Ventana y ciclo de vida

Las operaciones minimizar, maximizar/restaurar, cerrar, arrastrar y consultar estado usan exclusivamente las API tipadas oficiales de ventana mediante un adaptador único. Rust intercepta el cierre cuando `lifecycle.close_action=unset` y emite `lifecycle:close-decision-required`; la UI responde con:

- `resolve_first_close({ action: "exit" | "tray" | "dismiss" }) -> void`
- `get_window_state() -> WindowState`
- `set_window_state({ restored_x, restored_y, restored_width, restored_height, maximized, display_fingerprint }) -> WindowState`

La elección se persiste antes de ejecutar la acción; `tray` activa además `tray.monitoring_enabled`. `dismiss` (Esc o cierre del diálogo) no persiste nada y deja la ventana abierta. La geometría se captura desde eventos nativos, se valida en backend y no acepta coordenadas arbitrarias desde contenido web.

Cierre con operaciones en curso: Rust emite `lifecycle:close-blocked { reason: "guided" | "export" | "download" | "install" }`; la UI muestra el diálogo correspondiente («Detener y salir / Cancelar» para `guided`) y responde con `confirm_close({ stop_operation: boolean })`. Durante `install` el cierre se rechaza siempre.

Segunda instancia: el backend usa el plugin de instancia única; la nueva instancia enfoca la ventana existente (o la muestra si está en bandeja) y termina.

## Datos y restablecimiento

- `delete_monitoring_data({ confirmation_token }) -> DataOperationResult`
- `reset_application({ confirmation_token }) -> DataOperationResult`
- `get_storage_usage() -> { database_bytes, logs_bytes, total_bytes, session_count }`
- `open_logs_folder() -> void`
- `open_external_url({ target: "help" | "source" }) -> void`
- `get_technical_summary() -> TechnicalSummary`: versiones, protocolo, estado del colector,
  cobertura y métricas locales; sin identificadores ni registros en bruto.
- `get_third_party_notices() -> Notice[]`: avisos empaquetados, sin rutas recibidas de la UI.

Los tokens son efímeros y nacen de un diálogo local. `delete_monitoring_data` conserva las preferencias tipadas. `reset_application` borra todo el perfil local, desregistra inicio con Windows, elimina artefactos de actualización y provoca cierre/reinicio limpio. `DataOperationResult` informa por separado de los pasos `cleared` y `failed`; no acepta rutas ni URLs.

## Telemetría en vivo y cobertura

- `get_live_snapshot() -> LiveSnapshot`: último estado agregado (clasificación en vivo con `platform_kind`, `limit_severity` y si está dentro de la ventana de turbo; temperatura representativa y límite térmico efectivo; carga y núcleos activos; frecuencia activa y base por grupo; potencia y límite de potencia efectivo; potencial con mejor refrigeración si procede; contexto energético, frescura, estado del colector y nivel de cobertura). Sirve para el primer render; después la UI se suscribe al evento.
- `get_coverage() -> CoverageMatrix`: por magnitud (`temperature`, `thermal_headroom`, `load`, `active_clock`, `base_clock`, `power`, `power_limit`, `thermal_flag`, `prochot_flag`, `power_flag`, `current_flag`): disponible, calidad, sensor de origen, motivo de ausencia (`message_key`), el **nivel de cobertura** (`A`/`B`/`C`) con lo que permite concluir y la **confianza máxima alcanzable**. Incluye `low_level_access` ya traducido a la UI (`not_needed | available | installable | upgradable | denied | error`).
- `recheck_coverage() -> CoverageMatrix`: repite el descubrimiento sin reiniciar la sesión.
- `request_low_level_access({ action: 'install' | 'upgrade' | 'repair' }) -> AccessRequestResult`: abre el flujo explícito de instalación, actualización o reparación (UAC). `install` solo es válido con PawnIO ausente (`installable`), `upgrade` con una versión anterior a la mínima (`upgradable`, FR-089) y `repair` con PawnIO vigente que no entrega nivel A (`error`); en otro caso devuelve `coverage.access_not_installable`. `denied` (una política o el antivirus bloquea el acceso, `data-model.md`) no ofrece `repair`: no hay nada que reintentar, la interfaz enlaza a ayuda en su lugar (T161, 2026-09-21).
- `disable_advanced_access() -> CoverageMatrix`: desactiva el acceso avanzado en preferencias sin desinstalar PawnIO y fuerza la cobertura B/C.
- `get_core_detail() -> CoreDetail`: último frame por núcleo/grupo en memoria para la pantalla CPU y su tabla avanzada.
- `get_cpu_topology() -> CpuTopology`: devuelve los grupos P/E y los procesadores lógicos que el colector ha observado.

Eventos:

- `telemetry:snapshot { LiveSnapshot }` a la frecuencia del perfil activo (máximo 2 Hz hacia la WebView; el perfil `diagnostic` agrega).
- `telemetry:core-detail { CoreDetail }` a 1 Hz solo mientras la pantalla CPU está montada (`subscribe_core_detail({ enabled })`).
- `collector:state { state: "starting" | "running" | "degraded" | "restarting" | "stopped" | "failed", attempt?, message_key? }`.
- `coverage:changed { CoverageMatrix }` tras descubrimiento, reanudación o reinicio del sidecar.
- `power:context { source, scheme_hash, battery_percent? }` cuando cambia.

## Sesiones

- `list_sessions({ cursor?, limit }) -> SessionPage`: sesiones `passive`, `guided` e `imported` con estado ya mapeado a la UI (`active | completed | cancelled | incomplete | imported`), clasificación del informe si existe y `is_reference`. Las `replay` no se listan.
- `get_session({ session_id }) -> SessionDetail`: cabecera, contexto energético inicial/final, informe congelado (o provisional si está activa) y eventos.
- `delete_session({ session_id, confirmation_token }) -> void`: no permitido sobre la sesión activa.
- `get_report({ session_id }) -> DiagnosticReportView`: informe con textos como `message_key` + parámetros; la UI compone la prosa. Incluye `cooling_potential` (`band`, `low`, `high`, `method` y sus entradas) y, en sesiones guiadas, `guided_result` (rendimiento sostenido/inicial y desglose por causa) y la comparación «antes/después» si existe una referencia comparable.
- `reevaluate_report({ session_id }) -> DiagnosticReportView`: solo para `imported`; devuelve una segunda evaluación con el ruleset actual sin sobrescribir la original.
  Sin límite térmico efectivo por muestra (nunca se ha almacenado) ni catálogo original, así que el margen térmico y la meseta de niveles B/C no están disponibles en la reevaluación; la cobertura se deriva de lo que la sesión llegó a registrar, no de lo que el equipo anunciaba. Devuelve `report.reevaluation_not_imported` fuera de `imported` y `session.not_found` si no existe.
- `set_session_reference({ session_id, is_reference }) -> void`: marca o desmarca un diagnóstico guiado completado como referencia para comparaciones «antes/después»; devuelve `guided.reference_not_guided` si la sesión no es guiada.

Eventos:

- `session:changed { session_id, status }` al abrir, partir o cerrar una sesión.
- `report:frozen { session_id }` cuando un informe queda congelado.

## Exportación e importación

- `preview_export({ scope, format, anonymize }) -> ExportPreview`: `scope` = `{ kind: "session", session_id } | { kind: "report", session_id } | { kind: "range", session_id, start_ms, end_ms }`; `format` = `csv | json`. Devuelve campos incluidos/excluidos, tamaño estimado y nombre de fichero propuesto.
- `export({ scope, format, anonymize }) -> ExportResult`: abre el diálogo nativo de guardado desde Rust; la WebView nunca proporciona rutas. Devuelve `{ bytes, anonymized, warnings[] }` o `export.cancelled`.
- `cancel_export() -> void`.
- `import_session() -> ImportResult`: abre el diálogo nativo de apertura desde Rust; valida `schema_version`, migra si es anterior, rechaza si es futura (`import.schema_too_new`) y devuelve `{ session_id, schema_version, migrated, warnings[] }`.

Eventos:

- `export:progress { done, total? }`.
- `import:progress { phase: "reading" | "validating" | "migrating" | "storing" }`.

## Diagnóstico guiado

- `get_guided_preflight() -> GuidedPreflight`: comprobaciones (sensor crítico, reloj, alimentación, espacio en disco, generador disponible), `battery_state` (`ok | warning | blocked`) y los parámetros que se aplicarán (fases, duraciones según `guided.duration`, límites de parada).
- `start_guided({ skip_rest: boolean }) -> void`: falla con `guided.blocked_on_battery` si `guided.require_ac=true` y no hay CA; con `guided.already_running` si hay otra en curso; con `guided.operation_in_progress` si hay exportación/importación/instalación.
- `stop_guided() -> void`: cancelación inmediata (≤ 500 ms); los datos parciales se conservan como `cancelled`.

Eventos:

- `guided:phase { phase: "preflight" | "ready" | "rest" | "warming" | "steady_load" | "recovery" | "cancelling" | "cancelled" | "safety_stop" | "sensor_lost" | "error" | "result", elapsed_ms, remaining_ms?, reading?: { temperature, limit, headroom, active_clock, base_clock, throughput_ops_s }, reason_key? }`. `throughput_ops_s` es el trabajo medido por el generador.
- `guided:finished { session_id }`.

Ocultar la ventana o una suspensión del sistema provocan `stop_guided` implícito con motivo `guided.window_hidden` / `guided.system_suspend`.

## Bandeja y notificaciones

- `set_tray_paused({ paused }) -> void`: pausa o reanuda el muestreo desde el menú de bandeja; el estado se refleja en `collector:state`.
- `test_notification() -> void`: envía una notificación de prueba local (Ajustes → Bandeja).

Eventos:

- `tray:state { icon: "normal" | "warning" | "critical" | "unknown" | "disconnected", paused }`.
- `notification:opened { kind, session_id?, event_id? }` cuando el usuario pulsa una notificación; la UI navega a `Análisis` (térmica), `Sesiones` (prueba terminada) o `Ajustes` (actualización).

## Diagnóstico técnico y ayuda

- `get_technical_summary() -> TechnicalSummary`: versiones, protocolo, estado del colector, cobertura, últimos errores por código, métricas locales; sin identificadores. La UI ofrece «Copiar».
- `open_logs_folder() -> void`, `open_external_url({ target: "help" | "source" }) -> void`
  (solo destinos de una lista cerrada; nunca URLs libres), `get_third_party_notices() -> Notice[]`.
- `log_frontend({ events: FrontendLogEvent[] }) -> void`: única vía de registro de la interfaz (constitución XVII). Cada evento: `{ level: "error" | "warn" | "info" | "debug", code, target, msg, fields? }`, validado contra el esquema `log-event`; como máximo 60 eventos por minuto (el exceso se descarta y se registra el recuento). Solo se aceptan `debug` e `info` con `Registro detallado` activo. Rust añade `ts`, `component: "ui"` y la redacción.
- `Registro detallado` se gestiona con `set_preference({ key: "logging.detailed_until", value: true | false })`: `true` fija el vencimiento en ahora + 24 h y `false` lo desactiva; `get_preferences` devuelve la fecha de vencimiento o `null`.

## Ventanas de análisis

- `get_analysis_window({ session_id, start_ms, end_ms, target_points_per_track }) -> AnalysisWindow`

`target_points_per_track` se valida en backend y normalmente no supera 3.000. Rust elige `resolution_ms`, agrega si hace falta y devuelve hasta cuatro pistas y los eventos superpuestos. La respuesta incluye `is_aggregated` y conserva mínimos, máximos, primer/último valor, huecos, calidad material y límites exactos de eventos según `data-model.md`.

Un zoom provoca una nueva consulta con intervalo menor; la WebView no solicita ni recibe por defecto los siete días de muestras crudas. El comando no acepta SQL, nombres de tabla ni expresiones de agregación proporcionadas por la UI.

## Actualizaciones

- `get_update_state() -> UpdateState`: estado actual (`state`, `enabled`, `current_version`, `last_check`, `version`, `notes`, `downloaded`, `total`, `error_code`, `recoverable`); es lo que la interfaz lee al abrir Ajustes.
- `check_for_update({ manual: boolean }) -> UpdateState`: responde al instante con el estado y hace la comprobación en un hilo aparte; el resultado llega por eventos. Con las actualizaciones apagadas devuelve `update.disabled` sin ninguna petición de red.
- `download_update() -> UpdateState`: solo válido en `available`; la firma se verifica al terminar la descarga y solo entonces se llega a `verified`.
- `install_update() -> UpdateState`: solo válido en `verified`; con una operación en curso devuelve `update.blocked_by_running_operation` y `message_key` `update.blocked.<guided_test|export|import|data_operation>`.
- `set_updates_enabled({ enabled })` no es un comando aparte: se realiza con la preferencia tipada `updates.enabled` (`set_preference`), que apaga el actualizador, descarta lo descargado y detiene toda comprobación.

Eventos:

- `update:available { version, notes }`
- `update:progress { downloaded, total? }`
- `update:state-changed { state }`
- `update:error { message_key, code, recoverable }`

Con `manual=false`, el backend aplica la cadencia de 24 horas (se evalúa al arrancar y con un temporizador de 24 h mientras la aplicación está abierta). Con `manual=true`, permite reintento inmediato. `download_update` exige una versión comprobada y no alcanza `verified` sin firma Ed25519 válida. `install_update` exige `verified` y devuelve `update.blocked_by_running_operation { reason }` si hay diagnóstico, exportación, importación o borrado/restablecimiento activos. La URL del manifiesto, la ruta temporal y la clave pública no son parámetros de UI. No existe selección de canal.

Estados expuestos a la UI (`update:state-changed.state`): `idle | checking | up_to_date | available | downloading | verified | installing | error`. La UI muestra `Descargar` solo en `available` e `Instalar` solo en `verified`.

## Errores y seguridad

Todos los errores usan `{ code, message_key, path?, context? }` (constitución XIV): `code` estable en inglés `snake_case` con prefijo de dominio (`guided.already_running`), `path` con la ruta del campo cuando falla una validación (`["value", "quiet_hours", "start"]`) y contexto cerrado, sin rutas de fichero, excepciones, valores rechazados ni identificadores sensibles. Un parámetro inválido devuelve `validation.invalid_parameter` con su `path`. La interfaz valida este formato con Zod y traduce `message_key` desde su catálogo; nunca muestra mensajes crudos. Ningún comando acepta ejecutables, URLs, scripts o rutas proporcionadas por la WebView. Ningún comando acepta ejecutables, URLs, scripts o rutas proporcionadas por la WebView. Los permisos Tauri se reducen a los comandos y API de ventana estrictamente necesarios.
