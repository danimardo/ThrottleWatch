# ThrottleWatch — instrucciones para agentes

Punto de entrada común para Codex CLI, OpenCode y (mediante `CLAUDE.md`) Claude Code.
Fuente canónica del sistema de agentes: este fichero y `.agents/` (rules, skills, roles, meta).
Los ficheros de `.claude/rules`, `.claude/agents`, `.codex/agents` y las copias de skills en
`.claude/skills` se **generan** con `node scripts/agent/sync-adapters.mjs`; no se editan a mano.

## Idioma

- Con la persona usuaria, en documentos, specs, commits, PR, reglas y skills: **español**, con
  ortografía y tildes correctas.
- Identificadores, comentarios de código, registros, claves de contrato y de CSV/JSON: **inglés**.

## Qué es el proyecto

Aplicación de escritorio para Windows que diagnostica si una CPU pierde rendimiento por calor,
por potencia o por el propio equipo. Interfaz Svelte 5 + Vite en una WebView2 gestionada por
Tauri 2, backend Rust en el mismo proceso y colector .NET 10 (LibreHardwareMonitorLib) como
proceso auxiliar. **No usa SvelteKit ni Electron.** Un solo usuario, sin cuentas, sin red salvo
el actualizador voluntario. Licencia GPL-3.0.

Estado (2026-09-18): especificación, plan, tareas y sistema de diseño cerrados; **todavía no hay
código de aplicación** (`apps/`, `packages/`). El primer lote de trabajo es L00 en `tasks.md`.

## Fuentes de verdad (por orden de prioridad)

1. `.specify/memory/constitution.md` — marco innegociable (versión 1.5.3). Manda sobre todo lo demás.
2. `specs/001-cpu-thermal-diagnostics/spec.md` — requisitos, parámetros del motor y criterios de éxito.
3. `specs/001-cpu-thermal-diagnostics/plan.md` — arquitectura, excepciones registradas, estrategia de pruebas.
4. `specs/001-cpu-thermal-diagnostics/tasks.md` — tareas agrupadas en lotes L00–L21 con checkpoints `CHK-Lxx`.
5. `specs/001-cpu-thermal-diagnostics/{research,data-model,ux-visual-spec}.md` y `contracts/`.
6. `historias.md` — historias de usuario y, al final, la estrategia integral de testing.
7. `design/AGENTS.md` — manual del sistema de diseño (componentes, props, tokens). Solo cuando se
   trabaje con la interfaz; es largo (80 KB): ábrelo por secciones.

Si dos fuentes se contradicen, gana la de número menor y se anota la contradicción; nunca se
resuelve en silencio.

## Metodología

- El proyecto sigue **Spec Kit**: `specify → clarify → plan → tasks → analyze → implement`.
  Las skills `speckit-*` están en `.claude/skills` (Claude, `/speckit-x`) y en `.agents/skills`
  (Codex, `$speckit-x`). No se reescriben a mano: las mantiene la CLI `specify`.
- La implementación se hace **por lotes** (`tasks.md`, «Lotes y checkpoints»): plan de pruebas del
  lote → implementar → completar tests → ejecutar solo la suite afectada → `CHK-Lxx`. No se abre un
  lote con tests obligatorios pendientes del anterior. Detalle en `historias.md` § 7–9 y § 35–36.
- TDD obligatorio donde lo exige la constitución (motor, potencial, paradas de seguridad,
  anonimización, migraciones, transacciones destructivas, actualizador).
- Toda tarea terminada se marca `[x]` en `tasks.md`; todo requisito nuevo se traza en
  `traceability.md` cuando exista (T009a).

## Comandos verificados hoy

Solo existen en el sistema de diseño; la aplicación aún no tiene scripts.

```text
cd design/harness && npm ci && npm run check && npm run build
cd design/mockup  && npm ci && npm run check && npm run build
node scripts/agent/check-agents.mjs      # coherencia del sistema de agentes
node scripts/agent/sync-adapters.mjs     # regenera los adaptadores
```

Los comandos de `quickstart.md` (`pnpm test`, `pnpm test:e2e`, `cargo nextest`…) son **previstos**,
no verificados, hasta que L00 los cree. No inventes comandos: léelos de `package.json`,
`Cargo.toml` o `*.csproj` cuando existan.

## Reglas transversales

- **Constitución:** no se modifica sin autorización expresa. Si una tarea la contradice, se
  detiene el trabajo y se propone una enmienda (`speckit-constitution`).
- **Specs históricas:** nunca se borran ni se reescriben; se amplían o se enmiendan con fecha.
- **Git:** sin `commit`, `push`, ramas, `rebase`, `amend`, tags ni PR salvo petición explícita.
  Nunca `--no-verify`, `reset --hard` ni `push --force`. Los commits terminan con la línea de
  coautoría que indique la herramienta.
- **Secretos:** no leer, mostrar ni copiar `.env*`, `*.pem`, `*.pfx`, `*.key` ni tokens. Las
  variables de entorno se documentan en `.env.example` (constitución XV).
- **Validación en fronteras** (XIV), **sin `console.*` ni `println!`** fuera de los envoltorios de
  registro (XVII), **sin `process.env`** (XV), **sin SvelteKit, Tailwind, ORM ni telemetría**
  (tecnologías prohibidas de la constitución).
- **Sistema de diseño:** las pantallas conectan componentes de `design/` mediante adaptadores; no
  se recrean ni se copian estilos. Si falta una pieza, se añade en `design/` con ejemplo y harness.
- **Verificación honesta** (XVI): ejecuta las comprobaciones antes de declararlas superadas y
  registra orden, código de salida y avisos. Nunca digas que algo pasó si no lo ejecutaste.
- **Alcance:** no corrijas problemas ajenos a la tarea; anótalos. No amplíes permisos de agente
  para sortear una limitación.

## Definición de terminado

Una tarea está terminada cuando cumple su criterio en `spec.md`, pasa la suite del lote sin
errores ni avisos, respeta las puertas de calidad de la constitución aplicables y queda marcada
en `tasks.md`. Antes de cerrar un lote, ejecuta el revisor de solo lectura
`revisor-constitucion` (`.agents/roles/`) o la skill `cierre-de-lote`.

## Herramientas de agentes

| Herramienta | Lee | Skills | Agentes |
|---|---|---|---|
| Claude Code 2.1.x | `CLAUDE.md` → importa este fichero; `.claude/rules/` | `.claude/skills/` | `.claude/agents/` |
| Codex CLI 0.154 | este fichero (límite 32 KiB por conjunto) | `.agents/skills/` | `.codex/agents/*.toml` |
| OpenCode 1.17 | este fichero; `opencode.json` | `.claude/` y `.agents/skills` | `.opencode/agents/` (no configurado) |

Capacidades verificadas y fecha en `.agents/meta/agent-capabilities.yaml`. Cuando cambie la
versión de una herramienta, ejecuta `node scripts/agent/detect-capability-changes.mjs` y revisa
solo el adaptador afectado; no regeneres nada automáticamente.
