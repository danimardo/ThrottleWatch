@AGENTS.md

## Claude Code

- Las reglas con alcance por rutas están en `.claude/rules/` (generadas desde `.agents/rules/`).
- Skills del proyecto: `/speckit-*` (Spec Kit), `/cierre-de-lote` y `/verificar-agentes`.
- Subagente de solo lectura: `revisor-constitucion` (`.claude/agents/`), para cerrar lotes y antes de una PR.
- `.claude/settings.json` pide confirmación antes de editar la constitución o hacer `git push`; deniega leer o editar `.env*` (salvo `.env.example`), `*.pem`, `*.pfx`, `*.key` y `*.p12`, y deniega `git push --force`, `--no-verify` y `reset --hard`. No lo relajes para avanzar más rápido. Las denegaciones de `Read` no cubren todas las vías de Bash: es una barrera parcial.
- Los comentarios HTML de los adaptadores generados no llegan al contexto; el aviso «GENERADO» sirve a las personas.
