# Tareas: diagnóstico térmico de CPU

**Entrada**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `ux-visual-spec.md`, `contracts/`  
**Pruebas**: obligatorias según constitución; el motor debe desarrollarse contra trazas antes de conectarlo a hardware real.

Formato: `[ID] [P?] [Historia?] Descripción con ruta`. Las tareas de test indican entre paréntesis los FR/NFR/SC que cubren (puerta 1 de la constitución; matriz en `traceability.md`, T009a).

**Regla del sistema de diseño:** las pantallas **conectan** componentes de `design/` (copiados por T024) mediante adaptadores; no los recrean. Si una pantalla necesita una pieza que `design/` no tiene, se añade antes en `design/` con justificación, ejemplo y harness (como T118–T133), nunca en `apps/desktop/src/`.

## Fase 1 — Inicialización

- [ ] T001 Crear monorepo y estructura definida en `plan.md`.
- [ ] T002 Inicializar Tauri 2 + Svelte + TypeScript estricto en `apps/desktop/`.
- [ ] T003 [P] Crear solución .NET y proyecto `sensor-agent` en `apps/sensor-agent/`.
- [ ] T004 [P] Crear crates/módulos Rust `ipc`, `storage`, `diagnostics` y `export` en `apps/desktop/src-tauri/src/`.
- [ ] T005 [P] Configurar formatos, lint y análisis estático para TypeScript, Rust y C#.
- [ ] T006 Configurar scripts raíz de build/test/dev y fijar gestores/versiones en archivos de bloqueo.
- [ ] T007 [P] Configurar CI Windows x64 con cachés y artefactos de pruebas, sin requerir sensores reales.
- [ ] T008 [P] Crear inventario de licencias y plantilla de `THIRD-PARTY-NOTICES`.
- [ ] T009 Documentar decisiones ADR iniciales: sidecar, SQLite, `AnalysisChart` SVG, agregación temporal y motor por niveles de cobertura (mesetas, ventana de turbo, techo de potencia) en `docs/adr/`.
- [ ] T009a Crear `specs/001-cpu-thermal-diagnostics/traceability.md` (FR/NFR/SC/escenario de aceptación → test, o «manual» con motivo) y un check de CI que falle si un requisito no tiene entrada.

## Fase 2 — Fundamentos bloqueantes

- [ ] T010 Implementar tipos de contrato v1 compartidos a partir de `contracts/telemetry.schema.json`.
- [ ] T011 [P] Crear fixtures canónicos de handshake, capabilities, sample y error en `packages/contracts/fixtures/`.
- [ ] T012 [P] Crear tests C# de serialización de fixtures en `apps/sensor-agent/Tests/Protocol/`.
- [ ] T013 [P] Crear tests Rust de deserialización/validación de fixtures en `apps/desktop/src-tauri/src/ipc/tests/`.
- [ ] T014 Implementar handshake con nonce, versión y secuencia en sidecar y Rust.
- [ ] T015 Implementar supervisor de sidecar con estado, timeout, EOF y backoff limitado en `src-tauri/src/ipc/supervisor.rs`.
- [ ] T016 [P] Crear sidecar falso/replay en `packages/trace-fixtures/tools/`.
- [ ] T017 Integrar LibreHardwareMonitorLib con solo CPU habilitada en `apps/sensor-agent/Collector/`.
- [ ] T018 Implementar catálogo original de hardware/sensores sin normalización de negocio.
- [ ] T019 [P] Ejecutar spike de lectura sin privilegios en matriz Intel/AMD y documentar `docs/spikes/sensor-access.md`; incluir la fiabilidad de `% Processor Performance` y `Processor Frequency` por procesador lógico frente a APERF/MPERF.
- [ ] T019a **[Puerta de viabilidad]** Verificar con el proveedor de acceso de bajo nivel instalado si el sidecar lee `0x64F`, `0x1A2` y `0x610` y limpia los bits de registro **sin UAC recurrente**; medir qué parte de la matriz alcanza nivel A, B o C; decidir mediante ADR entre los resultados (a), (b) o (c) de `plan.md` § Fase 0. Bloquea la fase 3.
- [ ] T019b [P] Grabar el corpus etiquetado inicial (Intel híbrido, Intel anterior, portátil con DTT/DPTF, sobremesa con límites abiertos, AMD Zen 4) según `research.md` § 15, con cargas multihilo, juego de pocos núcleos, AVX2 y reposo caliente.
- [ ] T020 [P] Ejecutar spike de redistribución, instalación y retirada del acceso bajo nivel en `docs/spikes/low-level-driver.md`.
- [ ] T021 Resolver la arquitectura de privilegios mediante ADR; bloquear empaquetado si contradice la constitución.
- [ ] T022 Crear SQLite, migración v1 y repositorios base en `src-tauri/src/storage/`.
- [ ] T023 [P] Implementar reloj monotónico, detección de huecos y modelo de calidad de muestra.
- [ ] T047a Implementar límites de sesión pasiva (FR-067): apertura, partición por hueco > 60 s / 24 h, congelación del informe y diagnóstico en vivo provisional. (Movida desde la fase 5: el hito de esta fase y el corte vertical necesitan sesiones.)
- [ ] T024 [P] Crear `scripts/design-sync` que copie `design/{components,icons,illustrations,lib,tokens,brand}` a `apps/desktop/src/design-system/` y un test de CI que falle si la copia difiere; prohibir por lint la importación de `design/examples/` y `design/harness/`. Integrar `tokens.css` desde la copia para claro/oscuro, modo sistema y movimiento reducido.
- [ ] T025 Crear shell de navegación y estados globales de colector en `apps/desktop/src/`.
- [ ] T026 Configurar catálogos completos español/inglés, resolución especial de locales y verificador de igualdad/sin literales.

**Hito:** handshake, catálogo y muestras sintéticas atraviesan sidecar → Rust → UI → SQLite.

## Fase 2b — Ampliación del sistema de diseño (decisiones de 2026-09-18)

Estas tareas modifican `design/` **antes** de la primera sincronización (T024) y deben pasar `npm run check` y `npm run build` del harness. Ninguna añade copy incrustado ni dependencias. **Completadas el 2026-09-18** (batch 12 del sistema de diseño; harness y mockup compilan sin errores ni avisos; verificación visual en `examples/ShellPieces.example.svelte` y `design/mockup`).

- [x] T118 [P] `SettingsScreen`: sustituir `minimizeToTrayOnClose` por `closeAction: 'unset' | 'exit' | 'tray'` con `SegmentedControl` y texto «sin decidir»; eliminar `anonymizedDiagnostics` y el canal de actualizaciones; retención fija a 4 opciones; añadir movimiento (`appearance.motion`), `onBattery`, bloques `advanced` plegados por sección (intervalo, detalle por núcleo, periodo de silencio, nivel de registro), espacio usado, `Exportar…`, `Probar notificación`, `Repetir introducción`, enlaces de Acerca de (`Documentación`, `Documentación en línea`, `Licencias`, `Resumen técnico`, `Abrir carpeta de registros`); sección Diagnóstico con duración, exigir CA, avisar al terminar y límites en solo lectura.
- [x] T119 [P] `SettingsScreen.updates`: estados `idle|checking|upToDate|available|downloading|verified|installing|error`; botones `Descargar` (solo `available`) e `Instalar` (solo `verified`, con `installDisabledReason`); descripción de comprobación «cada 24 h».
- [x] T120 [P] Sustituir `advancedAccessAvailable: boolean` por `advancedAccess: 'not_needed'|'available'|'installable'|'denied'|'error'` en `OnboardingFlow` (slide 5) y `SettingsScreen.sensors`; solo `installable` muestra el botón de instalar/reparar.
- [x] T121 [P] Crear `CoverageMatrix` (magnitud × disponible/calidad/origen/motivo, pie con confianza máxima y estado de acceso, acciones Volver a comprobar / Copiar resumen técnico) y su ejemplo.
- [x] T122 [P] Crear `ContextStrip` (CPU + topología, alimentación/plan, estado del colector con frescura, Ver cobertura) con estados fresco/obsoleto/desconectado y su ejemplo.
- [x] T123 [P] Crear `ExportDialog` (alcance, formato, anonimizar, campos incluidos/excluidos, tamaño y nombre propuestos) e `ImportResultDialog` (validación, migración, avisos) y sus ejemplos.
- [x] T124 [P] Crear `FirstCloseDialog` (dos opciones, texto de bandeja «y seguir midiendo», enlace a Ajustes) y `CloseBlockedDialog` (prueba/exportación/descarga/instalación).
- [x] T125 [P] Crear `WhatsNewCards` (pila de tarjetas con «Entendido» y acción opcional) y `TechnicalSummary` (texto monoespaciado + Copiar) y `LicensesScreen` (lista desplegable de avisos de terceros).
- [x] T126 [P] `GuidedDiagnosticScreen`: añadir fase `rest` (omitible) al stepper (6 pasos), bloque estructurado `whatWillHappen[]` en la intro, tiempo restante por fase, `batteryState='blocked'` explicado, acción «Usar como referencia» en resultado.
- [x] T127 [P] Shell: `BottomBar` con ítem `Más` y menú (Sesiones, Diagnóstico guiado, Ajustes); `Banner` global fijo bajo `TitleBar` para prueba en curso (con Detener) y para estado del colector degradado (Reintentar / Ver resumen técnico); documentar en `AGENTS.md`.
- [x] T128 [P] `SessionCard`/`SessionsScreen`: marca «Referencia», acciones Usar/Retirar referencia, botón `Importar…` en cabecera, `statusLabel` obligatorio para `incomplete`. `ReportScreen`: acciones Exportar / Usar como referencia / Re-evaluar con reglas actuales y aviso «Provisional · sesión en curso».
- [x] T129 Actualizar `design/README.md`, `AGENTS.md` y `design-system.json` con los componentes nuevos y las props cambiadas; actualizar `design/mockup` a los contratos nuevos; ejecutar harness `check` y `build`.
- [x] T130 [P] Material de vidrio y catálogo de animaciones en `design/` (tokens `--glass-*`/`--motion-*`, niveles `data-glass`, fondo `tw-ambient`, utilidades `tw-enter`/`tw-pop`, animaciones por componente) según `ux-visual-spec.md` § «Material de vidrio» y § «Animación»; harness y mockup con conmutadores de vidrio y movimiento. (2026-09-18)
- [ ] T131 [US9] Implementar la preferencia `appearance.glass` (`system|full|reduced|off`): lectura de `UISettings.AdvancedEffectsEnabled`, escucha en caliente, aplicación de `data-glass` y degradación automática por rendimiento con histéresis; test de que `off` y sin `backdrop-filter` mantienen contraste AA.

## Fase 2c — Revisión del motor en el sistema de diseño (2026-09-18)

- [x] T132 [P] Añadir la clasificación `platform_limited` a `lib/classification.ts` y un icono propio en `StatusIcon`; documentar en `AGENTS.md`.
- [x] T133 [P] Actualizar ejemplos y mockup: hero con gravedad y «Enfriar mejor» en lugar de «Rendimiento disponible», informe con potencial por techo de potencia y rendimiento guiado medido, cobertura con nivel A/B/C, prueba guiada con frecuencia frente a base y rendimiento en vivo; regenerar capturas del README.

## Fase 3 — Historia 1: estado térmico actual (P1)

- [ ] T027 [P] [US1] Crear pruebas del normalizador para Intel homogéneo/híbrido, AMD y legado.
- [ ] T028 [P] [US1] Implementar las reglas de selección de temperatura representativa (`package`/`Tdie` → máx. núcleos → `Tctl` con offset → `Tctl` sin offset) y margen en `sensor-agent/Normalization/`, según `spec.md` § Parámetros iniciales.
- [ ] T028a [P] [US1] Implementar en el sidecar la clasificación de grupos P/E/LP mediante `GetLogicalProcessorInformationEx` y CPUID (hoja 0x1A) con fallback `unknown`.
- [ ] T028b [P] [US1] Implementar en el sidecar (nivel A, Intel) la lectura de `MSR_CORE_PERF_LIMIT_REASONS` como descriptores separados `thermal_flag`, `prochot_flag`, `power_flag` y `current_flag` usando los bits de registro con limpieza tras cada lectura (lista cerrada de bits, constitución VIII); `MSR_TEMPERATURE_TARGET` (TjMax, TCC offset, límite efectivo) y `MSR_PKG_POWER_LIMIT` (PL1, PL2, Tau; límite efectivo si hay MMIO). Sin acceso, no emitir descriptores.
- [ ] T028b2 [P] [US1] Implementar en el sidecar (AMD) la lectura de THM/PPT/TDC/EDC desde la tabla PM del SMU solo para versiones de la lista permitida, y la tabla versionada `thermal-limits-v1` por familia.
- [ ] T028c [P] [US1] Implementar en Rust `active_clock` y `base_clock` por procesador lógico con los contadores PDH `% Processor Performance` y `Processor Frequency` como descriptores `host`/`derived`, con test contra trazas `derived-clock-only`.
- [ ] T028e [P] [US1] Implementar el cálculo del nivel de cobertura (A/B/C) y su techo de confianza, expuesto en `get_coverage` y en el snapshot.
- [ ] T028d [P] [US1] Implementar en Rust la lectura del contexto energético (`GetSystemPowerStatus`, `PowerGetActiveScheme`, `WM_POWERBROADCAST`) y su persistencia por frame; detectar reanudación y aplicar FR-065.
- [ ] T029 [US1] Implementar normalización de temperatura, carga, reloj, potencia y flags conservando metadatos originales.
- [ ] T030 [US1] Implementar agregación de snapshot y frescura en `src-tauri/src/telemetry/`.
- [ ] T031 [P] [US1] Conectar `StatusHero` del sistema de diseño (copia en `src/design-system/`) mediante un adaptador en `src/features/dashboard/`.
- [ ] T032 [P] [US1] Conectar `StatWidget` para temperatura (con límite efectivo), carga y núcleos activos, frecuencia activa frente a base, y potencia frente a su límite, con mini-tendencias.
- [ ] T033 [P] [US1] Conectar `CoverageMatrix` y `ContextStrip` a `get_coverage`, `telemetry:snapshot`, `collector:state` y `power:context`; calcular «confianza máxima alcanzable» y el enum de acceso avanzado en Rust.
- [ ] T034 [US1] Conectar snapshots reales/replay a la pantalla `Ahora` sin lógica de sensor en UI.
- [ ] T035 [US1] Añadir pruebas UI para niveles A/B/C, parcial, obsoleto, desconectado y CPU híbrida.

**Prueba independiente:** abrir modo replay y comprender el estado; los ausentes dicen “No disponible”.

## Fase 4 — Historia 2: detectar y explicar limitaciones (P1)

- [ ] T036 [P] [US2] Definir `ruleset-v1.json` con la tabla ordenada de clasificación, mesetas, ventana de turbo, umbral de núcleo activo, gravedad frente a base, ocupación de razones, bandas y techos de confianza de `spec.md` § Parámetros iniciales, más códigos explicables; test que verifique que el fichero coincide con la spec.
- [ ] T037 [P] [US2] Etiquetar el corpus (T019b) con los bits de razón como verdad de referencia y generar sus copias degradadas a niveles B y C; incluir los casos obligatorios de `research.md` § 15 (fin de turbo, equipo que empieza caliente, degradación lenta, DPTF, PROCHOT externo, Zen 4 por diseño, juego de pocos núcleos, EcoQoS).
- [ ] T037a [P] [US2] Tests de `windows.rs`: núcleos activos (umbral 80 %), inicio de carga (< 30 % → sostenida), ventana de turbo `max(Tau, 60 s)`, `turbo_end` (≥ 15 % en ≤ 5 s) y ventana deslizante 60/10 s. Deben fallar antes de T038. (FR-077, FR-081)
- [ ] T037b [P] [US2] Tests de `thermal.rs`, `power.rs` y `platform.rs`: ocupación de razones (incluida la equivalencia AMD de FR-069), mesetas, bajada progresiva del límite frente a escalón único (reglas 7 y 7b). Deben fallar antes de T039–T040a. (FR-069, FR-078, FR-079, FR-084)
- [ ] T037c [P] [US2] Tests del clasificador: un caso positivo y otro negativo por fila, precedencia entre filas contiguas, ausencia de `mixed_limit` en niveles B/C, gravedad (97 %, 30 s), techos de confianza por nivel, clasificación de sesión por tiempo acumulado con intervalo analizado, y determinismo (misma traza → mismo resultado). Deben fallar antes de T041. (FR-009, FR-010, FR-070, FR-080, NFR-009)
- [ ] T038 [US2] Implementar en `diagnostics/windows.rs` los núcleos activos, la detección de inicio de carga y ventana de turbo (`max(Tau, 60 s)`), el evento `turbo_end` y la ventana estable deslizante.
- [ ] T039 [US2] Implementar en `diagnostics/thermal.rs` la ocupación de `THERMAL` (nivel A) y la meseta térmica contra el límite efectivo (niveles B/C).
- [ ] T040 [P] [US2] Implementar en `diagnostics/power.rs` la ocupación de razones de potencia y corriente, la meseta de potencia y la tendencia del límite de potencia o del nivel de meseta a lo largo de la sesión.
- [ ] T040a [P] [US2] Implementar en `diagnostics/platform.rs` `platform_limited` con subtipos `chassis_thermal` y `external_prochot`, y sus recomendaciones.
- [ ] T041 [US2] Implementar el clasificador como función pura que recorre la tabla ordenada, la gravedad `boost`/`below_base` frente a la frecuencia base, la confianza con techo por nivel y las causas alternativas (incluidas EcoQoS/EPP/plan).
- [ ] T042 [US2] Implementar segmentación y fusión de `limit_event` persistentes, incluidos `platform` y los marcadores `turbo_end`, y la clasificación de sesión por tiempo acumulado (`class_durations_json`, intervalo analizado).
- [ ] T043 [P] [US2] Crear el adaptador diagnóstico → props de `StatusHero` (`evidenceLine`), `ReportScreen` (`evidence`, `alternativeCauses`) y `AnalysisScreen` (`AnalysisEvidence`), con nivel de cobertura, gravedad e intervalo analizado.
- [ ] T044 [US2] Generar la narrativa causal y conectarla a `CausalRail` solo cuando la secuencia esté sustentada; el fin del turbo nunca es un eslabón.
- [ ] T045 [US2] Ejecutar la regresión del corpus y fijar en CI SC-003, SC-004, SC-005, SC-016, SC-017 y SC-018; calibrar los pesos de la confianza y versionarlos con el ruleset.

**Prueba independiente:** reproducir cada traza (y sus copias degradadas) y obtener clasificación, gravedad, confianza y evidencias esperadas.

## Fase 5 — Historia 3: potencial con mejor refrigeración (P1)

- [ ] T046 [P] [US3] Crear tests del método de techo de potencia: fórmula, acotación por la frecuencia de turbo, rango `[0,5·g, 1,0·g]`, redondeo hacia fuera a múltiplos de 5 %, tramos, ausencia de cifra sin PL1 y tramo cualitativo solo con `below_base`.
- [ ] T047 [US3] Implementar `diagnostics/potential.rs` con el método de techo de potencia, solo en nivel A, fuera de la ventana de turbo y para clases térmicas, mixta o chasis.
- [ ] T048 [US3] Implementar agrupación P/E/LP sobre núcleos activos para frecuencia, gravedad y potencial.
- [ ] T049 [US3] Implementar `guided_result` y la comparabilidad «antes/después» (misma CPU, perfil, contexto energético y versión del generador); `set_session_reference` solo para sesiones guiadas.
- [ ] T050 [US3] Aplicar restricciones: no serializar ninguna cifra sin método `power_headroom` con entradas o sin `guided_result`.
- [ ] T051 [P] [US3] Conectar el bloque de potencial y de rendimiento guiado de `ReportScreen` y `StatusHero.performance` (tramo, rango, método y entradas; sostenido/inicial con desglose por causa).
- [ ] T052 [US3] Añadir pruebas negativas: turbo máximo como referencia, fin de turbo tratado como pérdida, núcleos inactivos en la media, decimales y rangos menores de 5 puntos.

**Prueba independiente:** una traza de nivel A limitada térmicamente muestra el tramo y el rango con su método; la misma degradada a B no muestra cifra y explica por qué; una sesión guiada muestra el rendimiento medido desglosado.

## Fase 6 — Historia 4: diagnóstico guiado (P2)

- [ ] T053 [P] [US4] Ejecutar spike comparando carga integrada y observación externa en `docs/spikes/guided-load.md`.
- [ ] T054 [US4] Aprobar mediante ADR el modo seguro; si no, implementar guía para carga externa reproducible con la degradación de FR-083 (sin cifra de rendimiento; US3-4/5 y antes/después diferidas).
- [ ] T055 [P] [US4] Modelar máquina de estados y persistencia parcial de sesión guiada.
- [ ] T056 [US4] Implementar preflight de sensores, alimentación (`guided.require_ac`), perfil, espacio en disco y generador; fases según `spec.md` (comprobación, reposo omitible, calentamiento, carga sostenida de 180/240/360 s, recuperación); **paradas de FR-085** (nunca por alcanzar el límite térmico); cancelación al ocultar ventana o suspender; atajo `Ctrl+Shift+X`; `guided.notify_on_finish`.
- [ ] T056a [US4] Implementar el generador de carga con bucle fijo y versionado que cuenta operaciones por hilo y segundo, perfil sin AVX-512 y perfil AVX2 documentado; publicar `throughput_ops_s` en cada muestra.
- [ ] T057 [US4] Implementar controlador de fases, cancelación y watchdog fuera de workers.
- [ ] T058 [P] [US4] Conectar `GuidedDiagnosticScreen` a `guided:phase`: consentimiento, progreso, temperatura/límite efectivo, frecuencia activa frente a base, rendimiento medido en vivo y `Detener ahora`.
- [ ] T059 [US4] Probar cancelación, sensor perdido, proceso padre ausente, batería, temperatura por encima del límite + 2 °C, frecuencia < 50 % de la base en el límite, y que alcanzar el límite térmico **no** detiene la prueba.

**Prueba independiente:** completar o cancelar una sesión sin dejar carga activa; obtener informe o motivo explícito.

## Fase 7 — Historia 5: análisis visual (P2)

- [ ] T060 [P] [US5] Integrar el benchmark reproducible de `AnalysisChart` y validarlo en hardware objetivo con 4 pistas, 3.000 puntos por pista y eventos superpuestos.
- [ ] T061 [US5] Implementar consulta/agregación por resolución temporal en Rust conservando extremos, huecos, calidad y fronteras de eventos; objetivo 2.000–3.000 puntos por pista y 10.000–12.000 totales.
- [ ] T062 [P] [US5] Integrar el `AnalysisChart` SVG del sistema de diseño con cuatro pistas, huecos reales, cursor común y recarga de mayor resolución al hacer zoom.
- [ ] T063 [P] [US5] Mapear `limit_event` → `AnalysisChart.events` según `data-model.md` (bandas térmicas, eléctricas, del equipo y mixtas con patrones accesibles; `turbo_end` como marcador).
- [ ] T064 [P] [US5] Conectar `CpuTopologyMap` y `CpuAdvancedTable` por núcleo/grupo, con alternancia temperatura/reloj sin perder la selección (FR-064); si hace falta virtualizar la tabla, se añade antes en `design/`.
- [ ] T065 [US5] Conectar selección temporal con panel de evidencia y exportación de rango.
- [ ] T066 [US5] Añadir pruebas visuales, rendimiento y accesibilidad para zoom, cursor/rango por teclado, tabla textual, huecos, calidad reducida y densidad alta.

**Prueba independiente:** inspeccionar una sesión y rastrear una conclusión hasta valores sincronizados.

## Fase 8 — Historia 6: bandeja y alertas (P2)

- [ ] T067 [P] [US6] Implementar lifecycle de ventana/bandeja y preferencia de monitorización, incluido que elegir bandeja en la primera X active `tray.monitoring_enabled`, la corrección atómica de FR-061 y la instancia única.
- [ ] T067a [P] [US6] Implementar menú de bandeja (Estado, Abrir, Pausar/Reanudar, Salir), clic para mostrar/ocultar y `set_tray_paused`.
- [ ] T068 [P] [US6] Diseñar en `design/brand/` los iconos de bandeja para normal, aviso, crítico, desconocido y desconectado (con fuente, variantes claro/oscuro y tamaños de Windows), y sincronizarlos con T024.
- [ ] T069 [US6] Implementar los cinco tipos de alerta (`thermal_confirmed`, `power_limited`, `platform_limited`, `collector_lost`, `guided_finished`); las de limitación solo con gravedad `below_base` (FR-080); persistencia ≥ 90 s, enfriamiento 30 min por tipo, periodo de silencio opcional y navegación al pulsar (`notification:opened`).
- [ ] T070 [US6] Integrar notificaciones Windows con texto prudente y acceso a sesión.
- [ ] T071 [US6] Implementar perfiles 5 s / 1 s / 500 ms, `sampling.on_battery` (`keep`/`low_power`/`pause`) y `sampling.per_core_history`.
- [ ] T072 [US6] Probar cierre de ventana, reinicio del sidecar y ausencia de alertas por picos breves.

**Prueba independiente:** minimizar, provocar una traza persistente y recibir una única alerta explicable.

## Fase 9 — Historia 7: exportación, importación y privacidad (P3)

- [ ] T073 [P] [US7] Definir esquema JSON de informe/exportación separado del IPC.
- [ ] T074 [P] [US7] Implementar exportador CSV por streaming y snapshot consistente.
- [ ] T075 [US7] Implementar informe JSON versionado con eventos, intervalo analizado y duración por clase, nivel de cobertura, gravedad, potencial (método y entradas), resultado guiado y reglas.
- [ ] T076 [US7] Implementar anonimización por allowlist y test de fuga de identificadores.
- [ ] T077 [US7] Implementar importador/migrador, reproducción de sesión sin hardware y `reevaluate_report` (evaluación nueva junto a la original; el resultado importado nunca se usa como referencia local).
- [ ] T078 [US7] Conectar `ExportDialog` (tres alcances) e `ImportResultDialog` a `preview_export`, `export`, `import_session` y sus eventos; CSV según formato fijado en `spec.md`.

**Prueba independiente:** exportar anónimo, validar esquema, importar y reproducir con el mismo diagnóstico.

## Fase 10 — Historias 8 y 9: onboarding, idioma, tema y ventana (P1)

- [ ] T079 [P] [US8] Implementar repositorio y migración de `onboarding_state` con versión, progreso, completado y omitido.
- [ ] T080 [P] [US8] Conectar `OnboardingFlow` a las cinco diapositivas y al glosario breve de ambos catálogos, sin literales visibles.
- [ ] T081 [US8] Conectar detección pasiva a la quinta diapositiva y a la salida anticipada hacia `Ahora`.
- [ ] T082 [US8] Implementar repetición desde Ayuda y `WhatsNewCards` versionadas independientes; omitir sin confirmación.
- [ ] T083 [P] [US9] Implementar resolución pura de locale `es/ca/gl/eu/ast/an → es`, resto incluido `pt → en`, y override manual.
- [ ] T084 [P] [US9] Implementar tema `system/light/dark`, escucha de Windows, preferencia de movimiento (`data-motion`) y montaje del fondo ambiental `tw-ambient` en el shell.
- [ ] T085 [P] [US9] Configurar ventana sin decoraciones y conectar el `TitleBar` del sistema de diseño con un adaptador único de Tauri.
- [ ] T086 [US9] Persistir rectángulo restaurado/maximizado (mínimo 480×600, inicial 1100×760) y recuperar geometría fuera de monitores activos; formato de números/fechas con `Intl` según idioma efectivo.
- [ ] T087 [US9] Añadir pruebas de onboarding, locales, expansión de texto, tema, barra, doble clic y geometría multimonitor.

**Prueba independiente:** una instalación limpia puede completarse u omitirse en español/inglés, y al reiniciar conserva preferencias y una ventana visible.

## Fase 11 — Historia 10: ajustes y ciclo de vida (P2)

- [ ] T088 [P] [US10] Implementar almacén tipado/versionado de preferencias y validación de dependencias.
- [ ] T089 [P] [US10] Conectar `SettingsScreen` (secciones y panel avanzado plegado) al almacén de preferencias según `ux-visual-spec.md`.
- [ ] T090 [US10] Implementar `FirstCloseDialog` y persistencia `exit/tray` (con `dismiss` sin persistir), `CloseBlockedDialog` para prueba/exportación/descarga/instalación, sin confundir minimizar con ocultar.
- [ ] T091 [US10] Integrar inicio con Windows en modo ventana/bandeja, desactivado por defecto y reversible.
- [ ] T092 [P] [US10] Implementar perfiles de muestreo y valores iniciales de avisos/retención/anonimización.
- [ ] T093 [US10] Implementar borrado de datos conservando preferencias y restablecimiento total como transacciones separadas con informe de fallos parciales (FR-063); `get_storage_usage`; `TechnicalSummary`, `LicensesScreen`, `open_logs_folder` y `open_external_url` con lista cerrada.
- [ ] T094 [US10] Probar reinicio, dependencias, confirmaciones, fallos parciales y valores de fábrica.

**Prueba independiente:** cada ajuste persiste y las dos acciones destructivas afectan exactamente a los datos documentados.

## Fase 12 — Historia 11: actualizaciones voluntarias firmadas (P3)

- [ ] T095 [P] [US11] Documentar ADR de endpoint fijo, plugin oficial, custodia Ed25519 y permisos mínimos.
- [ ] T096 [P] [US11] Crear estado persistente y máquina de estados del actualizador en Rust.
- [ ] T097 [US11] Implementar comprobación automática cada 24 h sin tráfico cuando está apagada.
- [ ] T098 [US11] Implementar `Buscar actualizaciones` como reintento manual independiente de la cadencia.
- [ ] T099 [US11] Implementar descarga con progreso, temporales seguros, descarte de parciales y verificación Ed25519, exponiendo el estado `verified` como paso separado.
- [ ] T100 [US11] Implementar confirmación de instalación, cierre ordenado y bloqueo por diagnóstico/exportación/importación/borrado con motivo.
- [ ] T101 [P] [US11] Conectar `SettingsScreen.updates` a la máquina de estados del actualizador y crear la notificación nativa enlazada a la versión disponible.
- [ ] T102 [P] [US11] Configurar workflow de GitHub Releases para generar manifiesto y artefactos firmados sin exponer la clave privada.
- [ ] T103 [US11] Probar cero red desactivado, cadencias, firma inválida, interrupción, bloqueo y aislamiento de errores.

**Prueba independiente:** ningún artefacto se descarga sin gesto ni se instala sin firma y confirmación independientes.

## Fase 13 — Endurecimiento y entrega

- [ ] T104 [P] Implementar retención (incluida `session` = borrado al salir), purga segura, mantenimiento limitado de SQLite, modo solo memoria con disco lleno y recuperación de base de datos dañada (FR-075).
- [ ] T105 [P] Implementar logs estructurados rotados (5 × 5 MB, nivel `logging.level`) y resumen técnico anónimo.
- [ ] T106 [P] Completar teclado, lector de pantalla, contraste, ambos idiomas y movimiento reducido; ejecutar la matriz de ambos temas y tamaños compacto/medio/expandido sobre todas las pantallas.
- [ ] T107 Optimizar arranque, memoria, consultas y render hasta cumplir NFR-001 a NFR-003.
- [ ] T108 Ejecutar sesiones de una hora y pruebas de suspensión/reanudación; medir el crecimiento del almacenamiento por perfil y verificar NFR-016 (puerta 6 de la constitución).
- [ ] T109 Ejecutar matriz de hardware real y publicar cobertura observada.
- [ ] T110 Completar modelo de amenazas, CSP y revisión de comandos Tauri/sidecar/actualizador.
- [ ] T111 Completar obligaciones MPL 2.0, avisos, código fuente cubierto y SBOM.
- [ ] T112 Construir instalador NSIS x64 por usuario sin elevación, con WebView2 bootstrapper, pregunta de conservar datos al desinstalar, reparación y rollback; instalador separado por máquina para el acceso de bajo nivel si el spike lo aprueba.
- [ ] T113 Configurar firma de binarios/instalador y actualización segura.
- [ ] T114 Redactar ayuda en español e inglés, significado de métricas y límites del diagnóstico.
- [ ] T115 Ejecutar `speckit.converge`, resolver divergencias y registrar decisión de release.
- [ ] T116 [P] Incorporar a CI `npm ci`, `npm run check` y `npm run build` en `design/harness/` para impedir regresiones del sistema de diseño.
- [ ] T117 Auditar la aplicación contra `design/components/` y `design/examples/`, eliminar duplicados visuales casi equivalentes y documentar cualquier excepción aprobada.

## Dependencias

```mermaid
flowchart TD
    SETUP["Fases 1–2\nFundamentos"] --> US1["US1 Estado actual"]
    US1 --> US2["US2 Diagnóstico"]
    US2 --> US3["US3 Estimación"]
    US1 --> US4["US4 Prueba guiada"]
    US2 --> US5["US5 Análisis visual"]
    US2 --> US6["US6 Alertas"]
    US3 --> US7["US7 Exportación"]
    SETUP --> US8["US8 Onboarding"]
    SETUP --> US9["US9 Idioma y ventana"]
    US6 --> US10["US10 Ajustes y ciclo de vida"]
    US7 --> US11["US11 Actualizaciones"]
    US4 --> HARD["Endurecimiento"]
    US5 --> HARD
    US6 --> HARD
    US7 --> HARD
    US8 --> HARD
    US9 --> HARD
    US10 --> HARD
    US11 --> HARD
```

La fase 2b (T118–T129) precede a T024 y a cualquier pantalla que consuma los componentes modificados. US4 puede desarrollarse como observación externa si el spike de carga integrada no supera la puerta de seguridad. US5 puede comenzar con replay mientras se completa hardware real. US6 y US7 pueden avanzar en paralelo una vez estabilizados eventos e informes.

## Estrategia de MVP

El primer MVP demostrable incluye fundamentos, US1–US3, US8 y US9: onboarding bilingüe, apariencia/ventana, panel actual, diagnóstico explicable por niveles de cobertura y potencial solo por techo de potencia. Si el calendario exige recorte, pueden aplazarse carga integrada, bandeja, exportación avanzada y actualizaciones, pero no los dos idiomas, el tema claro/oscuro ni las salvaguardas que impiden mostrar porcentajes sin evidencia.
