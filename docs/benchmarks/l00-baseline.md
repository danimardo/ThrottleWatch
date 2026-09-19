# Línea base de L00

Medición local ejecutada el 2026-09-18 en Windows con `npx pnpm@12.4.2`; cada comando terminó
con código 0. Los tiempos incluyen el arranque del ejecutable de pnpm.

| Suite | Tiempo |
|---|---:|
| Formato | 3,201 ms |
| Lint | 20,328 ms |
| Check Svelte/TypeScript | 8,588 ms |
| Tests Vitest | 13,293 ms |
| Build Vite | 5,965 ms |
| `verify:batch` completo | 31,605 ms |

El objetivo inicial del pipeline de PR es una mediana inferior a 15 minutos. CI conserva los
artefactos de cobertura y del informe Playwright cuando existen; la medición de la mediana de
varias ejecuciones se ampliará con el historial del workflow.
