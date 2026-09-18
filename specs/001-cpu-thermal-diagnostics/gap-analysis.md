# Análisis de huecos, contradicciones y asunciones

**Fecha**: 2026-09-18  
**Alcance revisado**: `historias.md`, `.specify/memory/constitution.md`, `spec.md`, `plan.md`, `research.md`, `data-model.md`, `ux-visual-spec.md`, `quickstart.md`, `tasks.md`, `contracts/*`, `checklists/requirements.md`, `design/README.md`, `design/AGENTS.md`, interfaces reales de `design/components/*` y `design/mockup/src/App.svelte`.  
**Propósito**: enumerar cada punto donde un agente que ejecute `plan → tasks → implement` tendría que inventar una decisión, o donde dos documentos se contradicen. Cada punto lleva una **decisión propuesta** para poder aceptarla o corregirla rápidamente y volcarla después en `spec.md`, `data-model.md` o los contratos.

Leyenda de severidad: **[A]** bloquea o desvía la implementación · **[B]** produce inconsistencias visibles · **[C]** detalle a cerrar.

---

## 1. Contradicciones entre el mockup / sistema de diseño y la especificación

El sistema de diseño se declara "fuente canónica" (constitución 1.1.0, NFR-014). Por eso cualquier divergencia entre sus props y `spec.md` acaba implementándose tal como está en el componente, no como dice la spec. Estas son las que he encontrado:

| # | Sev. | Dónde | Qué dice el mockup / componente | Qué dice la spec | Decisión propuesta |
|---|---|---|---|---|---|
| 1.1 | **A** | `SettingsScreen.privacy` — "Diagnósticos anónimos: comparte fallos de la app sin datos identificables" y `about.links` "Enviar un diagnóstico" | Implica **telemetría de fallos por red** | FR-029, FR-058, constitución V: la única red del MVP es el actualizador opt-in | Eliminar ambos del mockup. `privacy.anonymize_exports` gobierna solo exportaciones a fichero. Si se quiere envío de fallos en el futuro, será una feature nueva con su propia spec. |
| 1.2 | **B** | `privacy.dataRetentionOptions` = 30 d / 90 d / 1 año / sin límite | NFR-007, FR-… : `session`, `1d`, `7d`, `30d`; 7 d inicial | Ajustar el ejemplo y el mockup a los cuatro valores de la spec. |
| 1.3 | **A** | `updates.channel` (Estable / Beta) | FR-058: endpoint fijo, no configurable | Decidir: **(a)** eliminar canal (recomendado para MVP); **(b)** mantener, en cuyo caso la spec debe añadir `updates.channel` a preferencias y definir el segundo endpoint/manifiesto. |
| 1.4 | **A** | `updates.status` = `idle/checking/upToDate/available/downloading/installing/error` y botón "Instalar y reiniciar" | FR-054: detectar, **descargar** e **instalar** son tres gestos separados; existe estado `verified` (`download_state`) | Añadir al componente los estados `downloaded`/`verified` y dos botones distintos: `Descargar` (en `available`) e `Instalar` (solo en `verified`). El botón de instalar debe admitir `installDisabledReason` (bloqueo por diagnóstico/exportación, FR-057). |
| 1.5 | **C** | `updates.autoCheckDescription` "Comprueba si hay una versión nueva **al iniciar**" | FR-053: como máximo una vez cada 24 h | Corregir texto de ejemplo. Definir además si la comprobación automática ocurre **al arrancar si han pasado 24 h** o **con temporizador mientras la app está abierta** (propongo ambas: al arrancar y cada 24 h en ejecución). |
| 1.6 | **A** | `general.minimizeToTrayOnClose: boolean` | FR-046 / `lifecycle.close_action` = `unset` / `exit` / `tray` + diálogo de primera X | El componente necesita un control de tres estados (o dos opciones + texto "Aún no decidido; se preguntará al cerrar"). Ver también 3.7 sobre qué ocurre si `close_action=tray` con `tray.monitoring_enabled=false`. |
| 1.7 | **B** | `diagnostics` = duración por defecto (5/10/20 min), avisar al terminar, exigir alimentación conectada | ux-visual-spec: "límites de seguridad y explicaciones"; `data-model.md` no tiene ninguna clave para esto | Decidir qué existe en Ajustes → Diagnóstico y añadir las claves a `user_preferences`. Propuesta: `guided.duration` (`short`/`standard`/`long`), `guided.require_ac` (bool, `false`), `guided.notify_on_finish` (bool, `false`, depende de `notifications.enabled`), y **solo lectura** de los límites de seguridad (temperatura de parada, tiempo máximo) sin permitir subirlos. |
| 1.8 | **B** | Faltan en `SettingsScreen` | FR-043 movimiento reducido; HU-06 pausa/reducción en batería; `notifications.quiet_period`; detalle por núcleo; espacio usado (HU-16); botón de exportación; `Repetir introducción`; `advanced.visible` (plegado de avanzados) | Ampliar las secciones del componente. El "plegado de avanzados" no existe en el componente: hoy `intervalo de muestreo` es visible siempre, contradiciendo FR-049. |
| 1.9 | **B** | `GuidedDiagnosticScreen.stepLabels` = Preflight / Calentamiento / Carga estable / Recuperación / Resultado | `plan.md` define 5 fases incluyendo **"reposo breve opcional para estabilizar"**; HU-04 exige explicar **carga, duración, sensores y condiciones de parada** antes de empezar | Decidir si existe la fase de reposo (propongo sí, opcional, 60 s, saltable). La pantalla `intro` necesita un bloque estructurado (lista de "qué va a pasar") en vez de solo `introTitle/introBody`. |
| 1.10 | **C** | `OnboardingFlow.onSkip` sale directamente | Historia 8, escenario 4: "cuando **confirma** la acción" vs ux-visual-spec "Omitir conduce a Ahora" (sin confirmar) | Los dos documentos de spec se contradicen entre sí. Propuesta: **sin confirmación** (omitir no es destructivo y el recorrido es repetible); corregir el escenario 4. |
| 1.11 | **B** | `AnalysisEvent.kind` = `thermal` / `electrical` / `mixed` | `limit_event.kind` = `thermal` / `power` / `current` / `mixed` / `unknown` | Definir mapeo en el adaptador: `power`+`current` → `electrical`; `unknown` → no se dibuja como banda (o banda neutra). Documentarlo en `data-model.md`. |
| 1.12 | **B** | `SessionStatus` = `active` / `completed` / `cancelled` / `incomplete` / `imported` | `monitoring_session.status` = `preparing/running/completed/cancelled/aborted/failed` y `kind` = `passive/guided/imported/replay` | Definir mapeo: `preparing`+`running` → `active`; `aborted`+`failed` → `incomplete` (con motivo en `statusLabel`); `kind=imported` → `imported`; `kind=replay` → ¿se lista o es solo modo dev? Propuesta: no se lista. |
| 1.13 | **B** | Acceso avanzado en UI es `advancedAccessAvailable: boolean` (y el mockup lo usa con dos significados distintos: "no hace falta" vs "no concedido") | `hello_ack.low_level_access.state` = `available/reduced/missing/denied/error/unknown` | Sustituir el booleano por un enum de UI con al menos: `not_needed`, `available`, `installable`, `denied`, `error`. `installable` es el único que muestra "Instalar/Reparar acceso avanzado". |
| 1.14 | **B** | Pantalla `Ahora` del mockup | ux-visual-spec "franja superior": nombre CPU, fuente de alimentación/perfil, **estado del colector y frescura**, "Ver cobertura" | El mockup solo tiene "Ver cobertura" (sin destino). No hay indicador de frescura ni de estado del colector, que son requisitos P1 (FR-019, HU-01). Hace falta un componente/franja para ello. |
| 1.15 | **B** | `BottomBar` en compacto = Ahora / Análisis / CPU / Diagnóstico | ux-visual-spec: "barra inferior **o menú**, manteniendo Ahora y **Detener prueba**" | En compacto no hay forma de llegar a Sesiones ni Ajustes, y no existe "Detener prueba" global. Definir: 4.º ítem "Más" con menú (Sesiones, Ajustes, Informe) y un `Banner`/botón fijo "Detener prueba" mientras haya prueba activa en cualquier pantalla. |
| 1.16 | **C** | Mockup importa `examples/*.example.svelte` (con literales en español) | AGENTS.md: los ejemplos no son componentes y llevan copy incrustado | Correcto para un mockup, pero `tasks.md` debería decir explícitamente que la app no puede importar `design/examples/` (T117 lo insinúa; conviene hacerlo explícito). |

---

## 2. Pantallas y componentes que la spec exige y el sistema de diseño no tiene

`design/README.md` afirma que "every screen the spec describes has a dedicated component". No es exacto. Faltan:

| # | Sev. | Qué falta | Requisito que lo pide |
|---|---|---|---|
| 2.1 | **A** | **Matriz de cobertura** (magnitud × disponible / calidad / procedencia / acción) | FR-023, research §10 "ficha local", botón "Ver cobertura" de `Ahora` |
| 2.2 | **A** | **Diálogo de exportación** con vista previa de campos incluidos/excluidos y selector CSV/JSON/anónimo | FR-028, HU-07, T078 |
| 2.3 | **B** | **Flujo de importación** (selección de fichero, validación de versión, error accionable, resultado) | FR-027, data-model "Migraciones" |
| 2.4 | **B** | **Tarjetas de novedades** tras actualización importante | FR-037, Historia 8 esc. 7 |
| 2.5 | **B** | **Diálogo de primera X** (composición concreta sobre `Dialog`: dos opciones + texto "puedes cambiarlo en Ajustes") | FR-046 |
| 2.6 | **B** | **Resumen técnico / diagnóstico técnico** copiable sin identificadores | plan "Observabilidad", NFR-004 "resumen técnico" en recuperación del sidecar |
| 2.7 | **B** | **Marcar sesión como referencia (baseline `user_marked`)** | `baseline.source = user_marked` en data-model; ninguna pantalla lo ofrece |
| 2.8 | **C** | **Franja superior de `Ahora`** (frescura, colector, alimentación) | ver 1.14 |
| 2.9 | **C** | **Pantalla de licencias / avisos MPL** | T111, constitución puerta 8. Decidir: pantalla en app o fichero abierto con el visor del sistema. |
| 2.10 | **C** | **Estado "sidecar desconectado / controlador ausente / error que requiere intervención"** como pantalla o banner global | ux-visual-spec "Estados esenciales"; existe `Banner`, pero no hay composición definida |

---

## 3. Huecos de dominio y del motor de diagnóstico

Estos son los puntos donde la spec dice "regla conservadora", "ventana configurada" o "umbral" sin dar el valor. Aunque el ruleset sea versionado (T036), **los valores iniciales v1 deben estar en la spec** o el agente los inventará.

| # | Sev. | Hueco | Decisión propuesta (valores iniciales v1, revisables) |
|---|---|---|---|
| 3.1 | **A** | **Umbrales del clasificador**: ¿qué es "próximo al límite"? ¿qué % de muestras con bandera térmica en la ventana corta? ¿qué caída de reloj es "sostenida"? ¿qué tolerancia de carga es "comparable"? | Proponer tabla en `spec.md` o `ruleset-v1.md`: margen ≤ 3 °C = "en el límite", ≤ 8 °C = "próximo"; bandera térmica activa ≥ 50 % de una ventana de 30 s = directa persistente; caída de reloj ≥ 8 % respecto a la mediana de los 60 s previos con carga ≥ 70 % y variación de carga ≤ 15 pp = correlacionada; `thermal_probable` exige ≥ 60 s. |
| 3.2 | **A** | **Bandas de confianza** `low/medium/high`: cómo se calcula el `confidence` 0..1 y dónde se corta | Definir factores (calidad de sensores, cobertura, duración, dispersión, antigüedad del baseline) y cortes: `< 0,45` low, `0,45–0,75` medium, `> 0,75` high. |
| 3.3 | **A** | **Estados del hero sin TjMax** ("reglas conservadoras cuando no exista") | Sin TjMax: `Temperatura alta` ≥ 85 °C; `Crítica` ≥ 95 °C; nunca "limitación" sin otra evidencia. Con TjMax: usar margen (3.1). Documentar que la escala del anillo es "aproximada" en ese caso. |
| 3.4 | **A** | **Temperatura representativa**: prioridad entre `package`, `max(core)`, `Tctl−offset`, `Tdie`, `CCD` | Prioridad: 1) package/Tdie directo; 2) máximo de núcleos; 3) Tctl con offset conocido; 4) Tctl sin offset (calidad `substitute`). Mostrar siempre en la tarjeta qué se está usando. |
| 3.5 | **A** | **Reloj efectivo vs sustituto** — hasta donde sé, `LibreHardwareMonitorLib` **no expone reloj efectivo** (sí lo hace HWiNFO) ni **banderas PROCHOT / PL1 / PL2 / EDP** como sensores. Si eso se confirma en T019, en la mayoría de equipos no habrá "señal directa" y la clasificación `thermal_confirmed` sería inalcanzable. | (1) Hacer explícita esta hipótesis en `research.md` riesgos. (2) Definir el sustituto: contador `\Processor Information(*)\% Processor Performance` × reloj base como "reloj efectivo derivado" (calidad `derived`), o el reloj por núcleo de LHM (calidad `substitute`). (3) Decidir si el sidecar puede leer MSR `IA32_THERM_STATUS`/`IA32_PACKAGE_THERM_STATUS` (Intel) y equivalentes AMD a través del acceso de bajo nivel de LHM para obtener la bandera térmica — y si no, aceptar que `thermal_confirmed` solo existe cuando el hardware/driver lo permita. (4) Decidir si se **permite porcentaje** con reloj `substitute`: propongo sí, con confianza máxima `medium`. |
| 3.6 | **B** | **Baseline aprendido**: criterios concretos (margen térmico mínimo, duración, carga mínima, número de ventanas), caducidad (`valid_until`) y qué lo invalida | Propuesta: ventanas de ≥ 120 s con carga ≥ 70 % estable, margen ≥ 15 °C, sin bandera eléctrica; mínimo 3 ventanas; caduca a 30 días o al cambiar `fingerprint`, versión del normalizador o contexto energético. |
| 3.7 | **A** | **Sesión pasiva**: cuándo empieza y termina. ¿Una por arranque? ¿Se corta al reanudar de suspensión, al reiniciar el sidecar, cada 24 h? ¿Qué significa `retention=session`? ¿Cuándo se congela su `diagnostic_report`? | Propuesta: nueva sesión pasiva al arrancar el muestreo, tras reanudación (hueco > 60 s) y tras reinicio del sidecar; máximo 24 h continuas (se parte). `session` = se borra al cerrar la app. El informe de una sesión pasiva se congela al cerrarla; mientras está activa, `Ahora` muestra un diagnóstico "en vivo" (no persistido). |
| 3.8 | **B** | **Contexto energético** (CA/batería, plan energético, modo OEM): FR-003 y plan lo exigen, pero **ningún mensaje del IPC lo transporta** y no se dice quién lo lee | Decidir que lo lee **Rust** vía Windows (`GetSystemPowerStatus`, `PowerGetActiveScheme`, `WM_POWERBROADCAST`) y se guarda como parte de `sample_frame.quality_flags`/columna `power_context`. Añadirlo a data-model. Modo OEM: fuera de alcance del MVP (declararlo). |
| 3.9 | **B** | **Suspensión/reanudación**: cómo se detecta y qué se hace (FR-001 "tras reanudar") | Rust escucha `WM_POWERBROADCAST`/`PBT_APMRESUMESUSPEND`; marca hueco, reinicia descubrimiento (`capabilities` nuevo), abre sesión nueva (3.7). |
| 3.10 | **B** | **Alertas**: lista cerrada de tipos, persistencia mínima, cooldown y `quiet_period` por defecto | Tipos MVP: `thermal_confirmed`, `power_limited` (opcional), `collector_lost`, `update_available` (canal aparte), `guided_finished` (si 1.7). Persistencia ≥ 90 s; cooldown 30 min por tipo; `quiet_period` por defecto: ninguno (opcional 22:00–08:00). Texto de cada notificación en ambos catálogos. |
| 3.11 | **B** | **Icono de bandeja**: menú contextual, clic izquierdo, doble clic | Propuesta: clic = mostrar/ocultar ventana; menú: Estado (texto), Abrir, Pausar/Reanudar muestreo, Salir. Añadir FR. |
| 3.12 | **B** | **Perfiles de muestreo**: intervalos y qué más cambia | `low_power` = 5 s, representativo, sin per-core persistido; `normal` = 1 s, per-group; `diagnostic` = 500 ms, per-core. En batería con `sampling.pause_on_battery` (nueva clave, `false`) el muestreo se **pausa**; alternativa `reduce` → `low_power`. |
| 3.13 | **B** | **Detalle por núcleo**: qué se muestra en vivo y qué se persiste | En memoria siempre `per_core` (para la pantalla CPU); en SQLite solo per-core en `diagnostic` o durante sesiones guiadas; el histórico per-core se agrega por grupo. |
| 3.14 | **A** | **Prueba guiada**: duración de cada fase, límite de parada (¿TjMax−2? ¿absoluto?), timeout del watchdog, si continúa con la ventana oculta, qué ocurre si el equipo se suspende, atajo de "Detener ahora" | Propuesta: comprobación ≤ 30 s, reposo 60 s opcional, calentamiento 90 s, carga 180 s (`standard`), recuperación 120 s. Parada: margen ≤ 1 °C o ≥ 100 °C absoluto, o 3 muestras consecutivas ausentes del sensor crítico, o 5 s sin latido. La prueba se cancela al ocultar la ventana o al suspender. Atajo: `Ctrl+Shift+X`. |
| 3.15 | **B** | **Anonimización**: allowlist concreta. ¿`display_name` de la CPU es identificador? ¿`power_profile`, `display_fingerprint`, nombre de usuario en rutas de exportación? | Publicar la lista en `contracts/export.schema.json`: se conserva `vendor`, `display_name` (modelo comercial, no serie), topología, versiones; se elimina hostname, usuario, series, MAC, rutas, `display_fingerprint`, `power_profile` GUID. |
| 3.16 | **B** | **Importación**: ¿se muestra el informe congelado o se re-evalúa con reglas actuales? ¿Qué pasa con baseline de otra CPU? | Ambas: se conserva el informe original y se ofrece "Re-evaluar con reglas actuales" mostrando ambas versiones. El baseline importado nunca se usa para la CPU local. |
| 3.17 | **B** | **Formato CSV**: ancho/largo, separador, decimales, zona horaria, nombre de fichero | Formato largo (`timestamp_utc, monotonic_ms, sensor_id, metric, scope, value, status, quality`), coma como separador, punto decimal, UTF-8 con BOM (Excel), UTC ISO-8601, nombre `throttlewatch_<tipo>_<fecha>_<id-corto>.csv`. |
| 3.18 | **C** | **Multi-socket / varias CPU** | Declarar fuera de alcance: se toma la primera CPU. |
| 3.19 | **C** | **Máquinas virtuales** (sin sensores) | Estado explícito "Entorno virtualizado: sin acceso a sensores" en cobertura. |

---

## 4. Huecos en contratos e IPC

| # | Sev. | Hueco | Decisión propuesta |
|---|---|---|---|
| 4.1 | **A** | `application-commands.md` solo cubre preferencias, onboarding, ventana, datos, `get_analysis_window` y actualizaciones. **No hay comandos ni eventos para**: snapshot en vivo, cobertura, lista/apertura/borrado de sesiones, exportar/importar, iniciar/detener prueba guiada, marcar baseline, resumen técnico, estado del colector, bandeja. | Completar el contrato antes de `/speckit.plan`. Como mínimo: `subscribe_snapshot` → evento `telemetry:snapshot`; `get_coverage`; `list_sessions`, `get_session`, `delete_session`; `export_session`, `import_session` (con diálogo nativo gestionado por Rust); `start_guided`, `stop_guided`, evento `guided:phase`; `mark_baseline`; `get_technical_summary`; eventos `collector:state`, `lifecycle:tray-state`. |
| 4.2 | **B** | IPC: no hay mensaje de **latido** ni mecanismo por el que el sidecar detecte la muerte del padre (research §8 lo exige) | Añadir `ping`/`pong` cada 2 s o, más simple, especificar que el sidecar termina al recibir EOF en `stdin` y además vigila el PID del padre. |
| 4.3 | **B** | IPC: `started` está en el schema pero no en `ipc-protocol.md`; `set_rate` no define rango; no se define si `capabilities` puede re-emitirse en caliente (p. ej. tras `set_rate` con `detail` distinto) | Documentar `started`; rango `set_rate` 250–10000 ms; `capabilities` se reemite tras `start` con `detail` distinto y tras reinicio. |
| 4.4 | **B** | IPC: `start.detail` (`representative/per_group/per_core`) no está en el schema; solo se validan `sample` y `error` | Ampliar `telemetry.schema.json` a todos los `type` (aunque sea en v1.1). |
| 4.5 | **B** | Contexto energético no viaja por IPC (ver 3.8) | Decidir lado Rust. |
| 4.6 | **C** | `hello.requested_locale` no tiene uso real definido | Eliminarlo o documentar para qué se usa exactamente. |
| 4.7 | **C** | Esquema de **exportación** (`export.schema.json`) no existe todavía aunque T073 lo prevé | Bien como tarea; conviene que el `spec.md` fije al menos `schema_version` inicial y compatibilidad hacia atrás. |

---

## 5. Huecos operativos y de escritorio

| # | Sev. | Hueco | Decisión propuesta |
|---|---|---|---|
| 5.1 | **B** | **Tamaño mínimo e inicial de ventana** (data-model dice "nunca inferior al mínimo de diseño" sin cifra) | Mínimo 480 × 600 px lógicos; inicial 1100 × 760 centrada. |
| 5.2 | **B** | **Cierre durante prueba/exportación/descarga** (caso límite listado, sin comportamiento) | Prueba: diálogo "Detener y salir / Cancelar". Exportación: esperar hasta 5 s y cancelar. Descarga: cancelar y descartar. Instalación: no se puede cerrar. |
| 5.3 | **B** | **Diálogo de primera X cerrado con Esc** | Permanece `unset`, la ventana no se cierra. |
| 5.4 | **A** | **`close_action=tray` con `tray.monitoring_enabled=false`**: FR-024 permite bandeja sin muestreo, lo que no aporta nada | Propuesta: elegir "Continuar en la bandeja" en la primera X **activa** `tray.monitoring_enabled` y el diálogo lo dice. Desactivar la bandeja después cambia `close_action` a `exit` atómicamente (igual que ya hace con `startup.mode`). |
| 5.5 | **B** | **Segunda instancia** | Enfocar la instancia existente (single-instance plugin) y salir. |
| 5.6 | **B** | **Disco lleno / SQLite dañada** | Disco lleno: pasar a solo memoria, `Banner` persistente, reintento cada 60 s. BD dañada: renombrar a `.corrupt-<fecha>` y crear nueva; avisar y ofrecer exportar el fichero dañado. |
| 5.7 | **B** | **Instalador**: tipo (NSIS/MSI/MSIX), por usuario vs por máquina, ruta de datos, si desinstalar borra datos | Propuesta: NSIS por usuario (sin UAC) para la app; el driver de bajo nivel, si el spike lo aprueba, en paso separado por máquina. Datos en `%LOCALAPPDATA%\ThrottleWatch`. Desinstalar pregunta si borrar datos. |
| 5.8 | **B** | **Logs**: ruta, tamaño de rotación, nivel, acceso desde la UI | `%LOCALAPPDATA%\ThrottleWatch\logs`, 5 × 5 MB, `info` por defecto, `debug` por preferencia avanzada; enlace "Abrir carpeta de registros" en Acerca de. |
| 5.9 | **B** | **Enlace "Documentación"** en Acerca de: abrir URL externa es tráfico de red por acción del usuario | Aceptable si es un gesto explícito. Definir que la ayuda esencial (T114) está **empaquetada** y "Documentación en línea" abre el navegador del sistema (no la WebView). |
| 5.10 | **B** | **Atajos de teclado** ("documentados" pero no listados) | Tabla mínima: `Ctrl+1..6` navegación; `Ctrl+,` Ajustes; `Ctrl+Shift+X` detener prueba; `Ctrl+E` exportar; `F1` ayuda; `Esc` cierra diálogos/tooltips. |
| 5.11 | **B** | **Formato de números y fechas**: ¿según idioma de la UI (es: `98,5`) o según región de Windows? ¿Solo °C? | Según idioma efectivo de la UI (`Intl` con `es-ES`/`en-US`). Solo °C en el MVP (declararlo). Fechas en zona horaria local, con `Intl.DateTimeFormat`. |
| 5.12 | **C** | **Origen del "idioma del sistema"**: idioma de visualización de Windows vs región. ¿Se usa solo el primero de la lista de idiomas preferidos? | Lista de idiomas de visualización (`GetUserPreferredUILanguages`), solo el primero. |
| 5.13 | **C** | **Origen del tema del sistema**: "modo de aplicación" vs "modo de Windows" | `AppsUseLightTheme` (modo de aplicación). |
| 5.14 | **C** | **Versión mínima de Windows 10** (WebView2, notificaciones) | 1809 (build 17763) o superior; WebView2 Runtime requerido (bootstrapper en instalador). |
| 5.15 | **C** | **Notificaciones**: clic en una alerta → qué pantalla; alerta de actualización → Ajustes (ya definido) | Alerta térmica → `Análisis` centrado en el evento. |
| 5.16 | **C** | **Bloqueo de instalación por diagnóstico/exportación**: ¿también por importación o por borrado de datos? | Sí para ambos; ampliar FR-057. |

---

## 6. Asunciones implícitas que deberían estar escritas en "Suposiciones" o "Fuera de alcance"

- Un solo usuario de Windows por perfil de datos; no hay sincronización entre usuarios ni equipos.
- Una sola CPU (socket); topologías multi-socket fuera de alcance.
- Solo °C; no hay preferencia de unidad.
- No se ejecuta como servicio de Windows; en bandeja sigue siendo un proceso de usuario.
- El sidecar .NET se ejecuta con los mismos privilegios que la UI (sin elevación) salvo que un spike demuestre otra cosa.
- Máquinas virtuales: cobertura reducida, no soportadas como caso principal.
- Detección de grupos P/E/LP: LHM no expone la clase de núcleo; el sidecar debe usar `GetLogicalProcessorInformationEx`/CPUID (leaf 0x1A) para clasificar. Debe constar como responsabilidad del sidecar y como riesgo.
- `thermal_confirmed` depende de que el hardware/driver exponga una bandera; en su ausencia el máximo alcanzable es `thermal_probable` (ver 3.5). La ficha de cobertura debe decirlo ("confianza máxima alcanzable").
- Windows 11 en ARM64 se publicará solo si `LibreHardwareMonitorLib`, PawnIO y .NET lo soportan; hasta entonces x64 únicamente.

---

## 7. Criterios de `historias.md` que no tienen FR en `spec.md`

| Historia | Criterio | Propuesta |
|---|---|---|
| HU-06 | "El icono diferencia normal, aviso, crítico, desconocido y desconectado" | Nuevo FR con los cinco estados y su mapeo a clasificaciones. |
| HU-06 | "El muestreo puede reducirse o pausarse en batería" | Nuevo FR + clave `sampling.on_battery` (`keep`/`low_power`/`pause`). |
| HU-12 | "Deshabilitar bandeja corrige una configuración incompatible" | Está en el contrato, no en spec. Añadir FR. |
| HU-13 | "Los controles dependientes permanecen visibles y explican por qué se deshabilitan" | Parcialmente en NFR-014; añadir FR explícito. |
| HU-14 | "Ambas acciones son transaccionales y comunican fallos parciales" | Añadir a FR-050/051. |
| HU-05 | "El mapa de núcleos permite alternar magnitud" | Añadir a FR-021. |
| HU-01 | "Reanudación tras suspensión conserva una interfaz útil" | Cubierto por casos límite; añadir escenario de aceptación. |
| HU-17 | "Los gráficos tienen alternativa textual, teclado y tabla del rango" | Cubierto por NFR-005 de forma genérica; añadir FR específico para `Análisis`. |

---

## 8. Erratas y detalles menores

- `quickstart.md` §1: `specify init thermalscope` → debería ser `throttlewatch`.
- `README.md` raíz: el árbol no menciona `design/`, que ahora es fuente canónica.
- `spec.md` Historia 8 esc. 4 vs `ux-visual-spec.md` "Primer inicio": confirmación de Omitir (ver 1.10).
- `data-model.md` `sample_value`: `value_real`/`value_bool` vs IPC `number`/`boolean`: nombres distintos; documentar que el mapeo es intencionado o unificar.
- `design/README.md`: "every screen the spec describes has a dedicated component" — matizar con la lista de §2.
- `ipc-protocol.md`: `started` no documentado; `set_rate` sin rango.
- `tasks.md` T024: "consumir o trasladar" el sistema de diseño es una decisión, no una tarea; conviene decidirla ya (propuesta: **copiar** `design/` a `apps/desktop/src/design-system/` con un script de sincronización y test de igualdad en CI, porque el harness fija Svelte 5.57 y el proyecto Tauri fijará su propia versión).
- SC-001 (90 % de usuarios en < 10 s) no es verificable en CI; marcarlo como criterio de pruebas con personas, no como puerta automática.

---

## 9. Sugerencia de orden para cerrar los huecos

1. Resolver los **[A]** de §1, §3 y §4 (1.1, 1.3, 1.4, 1.6, 3.1–3.5, 3.7, 3.14, 4.1, 5.4): sin ellos, `plan`/`tasks` producirán una app distinta de la especificada.
2. Volcar las decisiones en `spec.md` (nuevos FR y "Suposiciones"), `data-model.md` (claves de preferencias nuevas, `power_context`, mapeos de enums) y `contracts/application-commands.md` (comandos que faltan).
3. Actualizar el sistema de diseño (componentes de §2 y cambios de §1) **antes** de T024, ya que es la fuente canónica.
4. Pasar `/speckit.clarify` con este documento como lista de preguntas ya priorizadas y después `/speckit.analyze` para verificar coherencia.

---

## 10. Registro de decisiones (2026-09-18)

Todas las decisiones siguientes se han tomado con el propietario del producto y se han volcado en `spec.md`, `data-model.md`, `ux-visual-spec.md`, `plan.md`, `research.md`, `tasks.md` y `contracts/`.

| # | Decisión |
|---|---|
| 1.1 | Eliminar «Diagnósticos anónimos» y «Enviar un diagnóstico». Cero red salvo actualizador. `privacy.anonymize_exports` solo afecta a ficheros. |
| 1.2 | Retención: `session`, `1d`, `7d`, `30d`. |
| 1.3 | Sin canal de actualización. Único endpoint estable. |
| 1.4 | Tres gestos. Estados de UI: `idle/checking/upToDate/available/downloading/verified/installing/error`; botones `Descargar` (en `available`) e `Instalar` (solo en `verified`, con `installDisabledReason`). |
| 1.5 | Comprobación automática: al arrancar si han pasado ≥ 24 h y con temporizador de 24 h en ejecución. |
| 1.6 | Ajustes → General: `SegmentedControl` Salir / Bandeja; con `unset` ninguna opción marcada y texto «Se te preguntará al cerrar». |
| 1.7 | Preferencias `guided.duration` (`short`/`standard`/`long`, inicial `standard`), `guided.require_ac` (`false`), `guided.notify_on_finish` (`false`, depende de `notifications.enabled`). Límites de seguridad visibles, solo lectura. |
| 1.8 | Añadir a Ajustes: movimiento reducido (Apariencia), `sampling.on_battery` (Monitorización), periodo de silencio (Bandeja, avanzado), detalle por núcleo en histórico (Monitorización, avanzado), espacio usado + Exportar (Datos), Repetir introducción (Acerca de). Bloque «Avanzado» plegado **por sección**; el plegado no se persiste (se elimina `advanced.visible`). |
| 1.9 | Fase de reposo opcional (60 s, saltable) integrada como paso propio; intro con bloque estructurado `whatWillHappen[]`. |
| 1.10 | Omitir el onboarding **no** pide confirmación. Se corrige Historia 8 escenario 4. |
| 1.11 | Mapeo en adaptador: `power`+`current` → `electrical`; `unknown` → sin banda (solo en lista de eventos). |
| 1.12 | Mapeo en adaptador: `preparing`+`running` → `active`; `aborted`+`failed` → `incomplete` con motivo; `kind=replay` no se lista. |
| 1.13 | Enum de UI para acceso avanzado: `not_needed` / `available` / `installable` / `denied` / `error`. |
| 1.14 | Nuevo componente `ContextStrip` sobre el hero de `Ahora`. |
| 1.15 | `BottomBar`: Ahora, Análisis, CPU, Más (menú: Sesiones, Diagnóstico guiado, Ajustes). `Banner` fijo «Prueba en curso · Detener» bajo la barra de título en todas las pantallas y anchos mientras haya prueba activa. |
| 1.16 | La app no importa `design/examples/`. |
| 2.1 | `CoverageMatrix`: panel desde «Ver cobertura» en `Ahora` y embebido en Ajustes → Sensores. |
| 2.2/2.3 | `ExportDialog` único (alcance: sesión, informe o rango); importar desde cabecera de Sesiones con selector nativo y `Dialog` de resultado. |
| 2.4–2.7 | Crear `WhatsNewCards`, `FirstCloseDialog`, `TechnicalSummary` (Acerca de), acción «Usar como referencia» (baseline `user_marked`) en SessionCard e Informe con confirmación. |
| 2.9 | `LicensesScreen` con textos empaquetados. |
| 2.10 | `Banner` global de estado del colector bajo la barra de título; `Ahora` pasa el hero a `indeterminate`. |
| 3.1 | Umbrales v1 adoptados (ver `spec.md` § Parámetros iniciales). |
| 3.2 | Confianza = puntuación 0..1 por factores; cortes 0,45 / 0,75; techos `medium` sin bandera directa, con reloj sustituto/derivado o con cobertura parcial. |
| 3.3 | Sin TjMax: ≥ 85 °C «Temperatura alta», ≥ 95 °C «Crítica»; nunca «limitación» solo por temperatura. |
| 3.4 | Temperatura representativa: package/Tdie → máx. núcleos → Tctl con offset → Tctl sin offset. |
| 3.5 | Reloj efectivo derivado del contador `% Processor Performance` × base (calidad `derived`), leído por Rust; reloj LHM como `substitute`. Bandera térmica: intento de lectura MSR vía acceso de bajo nivel; sin ella, techo `thermal_probable`. Porcentaje permitido con reloj derivado/sustituto con confianza ≤ `medium`. |
| 3.6 | Baseline aprendido: ventanas ≥ 120 s, carga ≥ 70 % (±15 pp), margen ≥ 15 °C, sin bandera eléctrica, ≥ 3 ventanas; caduca a 30 d o por cambio de fingerprint/normalizador/contexto. Prioridad guided > user_marked > learned. |
| 3.7 | Sesión pasiva nueva al arrancar muestreo, tras hueco > 60 s y cada 24 h. Informe congelado al cerrarla; diagnóstico en vivo (provisional) mientras está activa. `retention=session` = se borra al salir. |
| 3.8/3.9 | Contexto energético lo lee Rust (Win32) y se persiste por frame (`power_context`). Reanudación → hueco + re-descubrimiento + sesión nueva. Modo OEM fuera del MVP. |
| 3.10/3.11 | Alertas: `thermal_confirmed`, `power_limited`, `collector_lost`, `guided_finished`; `update_available` por canal aparte. Persistencia ≥ 90 s, cooldown 30 min/tipo, silencio opcional. Bandeja: clic = mostrar/ocultar; menú Estado/Abrir/Pausar-Reanudar/Salir. Clic en alerta térmica → Análisis centrado en el evento. |
| 3.12/3.13 | `low_power` 5 s representativo; `normal` 1 s por grupo; `diagnostic` 500 ms por núcleo. `sampling.on_battery` = `keep`/`low_power`/`pause`, inicial `keep`. Per-core siempre en memoria; en SQLite solo en `diagnostic`, sesión guiada o con `sampling.per_core_history=true`. |
| 3.14 | Prueba: comprobación ≤ 30 s, reposo 60 s opcional, calentamiento 90 s, carga 180 s (`standard`; `short` 90 s, `long` 300 s), recuperación 120 s. Parada: margen ≤ 1 °C o ≥ 100 °C, 3 muestras sin sensor crítico, 5 s sin latido del generador. Ocultar ventana o suspender cancela. Atajo `Ctrl+Shift+X`. |
| 3.15–3.17 | Allowlist de anonimización, importación con «Re-evaluar con reglas actuales» y CSV largo (coma, punto decimal, UTF-8 BOM, UTC) según propuesta. |
| 3.18/3.19 | Multi-socket fuera de alcance; VM = estado explícito de cobertura. |
| 4.1 | Contrato de comandos completado (ver `contracts/application-commands.md`). |
| 4.2–4.4 | Sidecar termina con EOF en stdin o al desaparecer el PID padre (cada 2 s); sin ping. `set_rate` 250–10 000 ms. `capabilities` se reemite tras `start` con `detail` distinto y tras reinicio. Schema ampliado a todos los `type`. |
| 4.6/4.7 | Se elimina `requested_locale`. `export.schema.json` v1 fijado en spec. |
| 5.1–5.3 | Mínimo 480×600, inicial 1100×760 centrada. Cierre durante operaciones y Esc en primera X según propuesta. |
| 5.4 | Elegir «bandeja» en la primera X activa `tray.monitoring_enabled`; desactivar bandeja cambia `close_action` a `exit` atómicamente. |
| 5.5–5.8 | Segunda instancia enfoca la existente. Disco lleno / BD dañada según propuesta. NSIS por usuario; driver aparte por máquina; `%LOCALAPPDATA%\ThrottleWatch`; desinstalar pregunta si borrar datos. Logs 5×5 MB, `info`, «Abrir carpeta de registros». |
| 5.9–5.16 | Ayuda empaquetada + «Documentación en línea» en navegador del sistema. Atajos: `Ctrl+1..6`, `Ctrl+,`, `Ctrl+Shift+X`, `Ctrl+E`, `F1`, `Esc`. Números/fechas con `Intl` según idioma efectivo; solo °C. Idioma = primer idioma de visualización de Windows; tema = `AppsUseLightTheme`; Windows 10 1809+ con WebView2. FR-057 ampliado a importación y borrado. |
| §6 | Asunciones añadidas a `spec.md`; riesgo P/E/LP en `research.md`. |
| §7 | FR-059…FR-066 añadidos. |
| §8 | Erratas corregidas. T024 = copiar `design/` a la app con script de sincronización y test de igualdad en CI. |

### Aplicación al sistema de diseño y al mockup (2026-09-18)

Las tareas T118–T129 se han ejecutado sobre `design/` (batch 12) y `design/mockup` se ha actualizado para reflejar todas las decisiones: `historias.md` incorpora los criterios nuevos (HU-01 a HU-17 ampliadas; HU-18 Sesiones y HU-19 resiliencia nuevas). `design/harness` y `design/mockup` pasan `npm run check` (0 errores, 0 avisos) y `npm run build`.

### Material de vidrio y animaciones (2026-09-18, petición posterior)

Decisión del propietario: vidrio en todas las superficies con opacidad alta (legibilidad ante todo) y animaciones solo en transiciones y cambios de estado, con reposo calmado. Implementado como batch 13 del sistema de diseño (T130) y recogido en `ux-visual-spec.md` (§ «Material de vidrio», § «Animación»), `data-model.md` (`appearance.glass`), `spec.md` (FR-043b, NFR-001) y `plan.md` (spike de coste del vidrio en WebView2). Reglas enmendadas en `AGENTS.md`: 6 (elevación), 24–26 (vidrio y movimiento).

### Apariencia nativa, no de página web (2026-09-18, petición posterior)

Decisión del propietario: la aplicación no debe leerse como una página web dentro de la ventana. Se añade NFR-015 y se documenta en `ux-visual-spec.md` § «Interacción nativa, no de página web»: sin subrayado en hover/foco, sin cursor de mano salvo en la superficie de `AnalysisChart`, navegación mediante `<button>` y no `<a href>`. Se corrigieron en `design/` tres cursores de mano heredados de convención web (`LicensesScreen`, el disclosure «Avanzado» de `SettingsScreen`, y varios botones del `mockup`) a `cursor: default`; la única excepción que queda es la superficie interactiva del gráfico. Reglas añadidas: `tokens.css` (comentario normativo) y `AGENTS.md` regla 27.

---

## 12. Revisión del motor de diagnóstico (2026-09-18)

**Pregunta del propietario:** ¿la lógica para decidir, inferir o confirmar pérdida de rendimiento por temperatura es correcta o solo una estimación pobre? ¿Mejorar o descartar la aplicación?

**Diagnóstico de la versión anterior.** El marco (evidencia antes que afirmación, razones directas, no usar el turbo de caja, grupos P/E/LP) era correcto; las reglas concretas no:

| # | Fallo | Efecto |
|---|---|---|
| 12.1 | La regla «probable» (calor seguido de caída de reloj ≥ 8 %) reproduce la firma del fin del turbo PL2 → PL1 | Falsos positivos térmicos en casi cualquier Intel sin driver |
| 12.2 | La gestión térmica del fabricante (DTT/DPTF, STAPM) aparece como límite de potencia | Recomendación errónea de «no mejorar la refrigeración» en portátiles |
| 12.3 | Sin frecuencia base, el boost de diseño hasta el límite térmico (Zen 4, Raptor Lake) se veía como problema | Alarmas sin sentido en sobremesa |
| 12.4 | Sin TCC offset, el margen al límite era falso en muchos portátiles | Reglas que no saltan |
| 12.5 | Un único «bit térmico» mezclaba `THERMAL` y `PROCHOT` | «Confirmada» por señales externas (batería, cargador) |
| 12.6 | Detección de caídas en vez de estados; carga total ≥ 70 % | Equipos que ya empiezan calientes y juegos de pocos núcleos quedaban fuera |
| 12.7 | Muestreo del bit instantáneo a 1 Hz | Pérdida de episodios intermitentes |
| 12.8 | Porcentaje por razón de relojes frente a baselines aprendidos | Mezcla de turbo, tipo de carga y referencias sesgadas; rango con falsa precisión |
| 12.9 | La prueba guiada se detenía con margen ≤ 1 °C | No podía observar la limitación térmica que pretendía medir |

**Decisión: mejorar, no descartar**, con una puerta de viabilidad explícita.

| Cambio | Dónde |
|---|---|
| Niveles de cobertura A/B/C con techo de confianza | `spec.md` (Parámetros, FR-070, FR-082), `research.md` § 6 |
| Razones directas separadas (`THERMAL`, `PROCHOT`, potencia, corriente) con bits de registro | FR-069, FR-084, `ipc-protocol.md` |
| Límite térmico efectivo (TjMax − TCC offset) | FR-076 |
| Ventana de turbo excluida y evento `turbo_end` | FR-077 |
| Atribución por mesetas (lo que se queda clavado en su tope) | FR-078 |
| Clase `platform_limited` (chasis / PROCHOT externo) | FR-009, FR-079 |
| Gravedad `boost` / `below_base` frente a la frecuencia base; alertas solo en `below_base` | FR-009, FR-080 |
| Carga evaluada en núcleos activos | FR-081 |
| Potencial por techo de potencia `(PL1/P)^(1/3) − 1`, en tramos | FR-013, FR-015, `research.md` § 5 |
| Rendimiento **medido** por el generador en la prueba guiada; referencias solo guiadas | FR-072, FR-083, `data-model.md` (`guided_result`) |
| Parada de seguridad corregida | FR-085 |
| Validación con corpus etiquetado por razones directas y copias degradadas | SC-003–005, SC-016–018, `research.md` § 15 |
| Constitución 1.2.0 (principios III y VIII, puerta 4) | `.specify/memory/constitution.md` |
| Puerta de viabilidad: nivel A sin UAC recurrente | `plan.md` fase 0, T019a |
| Sistema de diseño: `platform_limited`, icono `device`, `severity` en `StatusHero`, eventos `platform`/`info`, props `method*` | `design/` (T132, T133) |

**Límites que siguen existiendo.** En AMD de consumo no hay razones de limitación documentadas: el techo realista es el nivel B salvo versiones de la tabla PM incluidas en la lista permitida. La relación cúbica potencia-frecuencia es de primer orden; el rango `[0,5·g, 1,0·g]` y la medición guiada la acotan. Si la puerta de viabilidad falla (nivel A inalcanzable sin UAC en cada arranque ni servicio aceptable), el producto se reposiciona como explicador prudente y se decide explícitamente si continuar.

