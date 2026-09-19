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

## Check de CI

El check inicial valida que este fichero existe y que todo requisito nuevo se añade a la tabla.
La validación semántica de rangos se ampliará en T009a cuando se creen los contratos y fixtures.
