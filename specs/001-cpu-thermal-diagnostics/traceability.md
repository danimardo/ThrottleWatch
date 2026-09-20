# Trazabilidad de requisitos

Este inventario se mantiene junto con los artefactos de la feature. Los requisitos que todavía no
tienen implementación se marcan como `pendiente` y deben recibir una prueba o justificación manual
antes del cierre del lote que los introduce.

| Requisito | Cobertura | Estado |
|---|---|---|
| FR-001–FR-086 | Tests de contrato, motor, integración y aceptación definidos por lotes en `tasks.md` | pendiente |
| FR-087–FR-090 | Instalación, elevación del sidecar, reutilización y degradación del acceso avanzado (aclaraciones 2026-09-19); tareas T151–T163 (`tasks.md` § Phase 14: Convergence) | pendiente |
| NFR-001–NFR-016 | Checks de plataforma, rendimiento, seguridad y accesibilidad definidos por lotes | pendiente |
| SC-001–SC-018 | Suites de aceptación y corpus etiquetado definidos por lotes | pendiente |
| SC-019–SC-020 | Instalación con una sola UAC y degradación en sesión sin pérdida de muestras; pruebas de integración/E2E en T153, T154 y T160–T161 | pendiente |

## Desglose verificado de T170

Estas filas detallan los requisitos que no deben quedar ocultos dentro de los rangos
globales anteriores. El estado describe la evidencia disponible en el árbol actual; no
convierte en aceptados los escenarios que siguen pendientes por hardware o por E2E nativo.

| Requisito | Evidencia verificable | Estado |
|---|---|---|
| FR-010 | `apps/desktop/src-tauri/src/diagnostics/` (clasificación con intervalo analizado), `apps/desktop/src-tauri/src/commands/mod.rs` (ventana de análisis y límites temporales) y `T037c`, `T061`, `T169` en `tasks.md` | verificado en unitarias; E2E nativo pendiente |
| FR-018 | `apps/desktop/src-tauri/src/diagnostics/guided.rs`: `#![forbid(unsafe_code)]`; `T056a`/`T059` y la prueba `the_threaded_generator_counts_real_work_per_thread_and_stops_promptly` | verificado por compilación y pruebas |
| FR-020 | `apps/desktop/src-tauri/src/commands/mod.rs` y `src/features/analysis/Analysis.svelte`: pistas por identificador de catálogo, eventos y resolución de `session_id: 'latest'`; `T061`–`T063`, `T065`, `T169` | verificado en backend y E2E frontend |
| FR-066 | `src/features/analysis/Analysis.svelte` y `T-E2E-05`: cursor, selección de rango, resumen textual y tabla alternativa; `T066` mantiene abierta la auditoría `axe-core` y ambos temas/idiomas | parcial; accesibilidad completa pendiente |
| FR-085 | `apps/desktop/src-tauri/src/diagnostics/guided.rs`: `is_over_limit`, `is_severely_throttled` y pruebas de fronteras 2,0/2,1 °C, 49/50 %; `T056`, `T057`, `T059`, `T166` | verificado en lógica y pruebas; E2E térmico de hardware pendiente |

### Parámetros de carga guiada

`spec.md` § «Parámetros iniciales» usa el identificador conceptual `guided.load_s` y
describe sus tres perfiles. El fichero ejecutable `apps/desktop/src-tauri/resources/ruleset-v1.json`
y el cargador Rust usan las claves concretas `guided.load_short_s`,
`guided.load_standard_s` y `guided.load_long_s`, que son las que consume
`GuidedProfile::load_seconds`. La diferencia es intencionada: la primera es la
notación agrupada de la especificación y las segundas son parámetros versionados
por perfil; no se debe introducir una cuarta clave `guided.load_s` en el código.

## Check de CI

El check inicial valida que este fichero existe y que todo requisito nuevo se añade a la tabla.
La validación semántica de rangos se ampliará en T009a cuando se creen los contratos y fixtures.
