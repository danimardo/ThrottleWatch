---
name: verificar-agentes
description: Verifica y mantiene el sistema de instrucciones para agentes de ThrottleWatch (AGENTS.md, .agents/, adaptadores de Claude Code y Codex). Úsala tras editar AGENTS.md o .agents/, cuando una herramienta de agentes cambie de versión, o si un agente no parece cargar las instrucciones. No la uses para tareas del producto.
---

# Verificar el sistema de agentes

Fuente canónica: `AGENTS.md` y `.agents/{rules,skills,roles,meta}`. Adaptadores generados:
`.claude/rules/*.md`, `.claude/skills/{cierre-de-lote,verificar-agentes}`,
`.claude/agents/revisor-constitucion.md`, `.codex/agents/revisor-constitucion.toml`.
Manuales: `CLAUDE.md`, `.claude/settings.json`. Propiedad de Spec Kit (no tocar):
`.claude/skills/speckit-*`, `.agents/skills/speckit-*`, `.specify/**`.

## Comprobación rutinaria

```text
node scripts/agent/check-agents.mjs
```

Falla si: `AGENTS.md` supera 200 líneas; un adaptador generado difiere de lo que produciría
`sync-adapters`; una skill o regla canónica no tiene `name`/`description` o `paths` válidos;
`CLAUDE.md` no importa `@AGENTS.md`; falta un fichero generado.

Si difiere un adaptador, **no lo edites**: corrige la fuente en `.agents/` o `AGENTS.md` y
ejecuta `node scripts/agent/sync-adapters.mjs`.

## Cambio de versión de una herramienta

```text
node scripts/agent/detect-capability-changes.mjs
```

Compara `claude --version`, `codex --version` y `opencode --version` con
`.agents/meta/agent-capabilities.yaml`. Si una versión cambió:

1. No regeneres nada automáticamente.
2. Consulta la documentación oficial de esa versión (rutas de skills, rules, agents, permisos).
3. Clasifica el impacto: SIN IMPACTO · REVISIÓN RECOMENDADA · CAMBIO NECESARIO · INCOMPATIBILIDAD.
4. Propón el cambio a la persona usuaria; solo tras su aprobación actualiza el adaptador y la
   entrada `validated_version`/`validated_at` en `agent-capabilities.yaml`.

## Comprobar la carga en cada herramienta

- **Claude Code:** `/context` debe listar `CLAUDE.md`, `AGENTS.md` (importado) y las reglas sin
  `paths`; `/agents` debe mostrar `revisor-constitucion`; `/cierre-de-lote` debe existir.
  Requiere reiniciar la sesión tras cambiar `settings.json`, rules o agents.
- **Codex CLI:** `codex debug prompt-input` debe incluir el contenido de `AGENTS.md` y las skills
  de `.agents/skills`. Los agentes de `.codex/agents/` se comprueban con `/agents` (según versión).
- **OpenCode:** `opencode debug skill` lista skills; requiere configuración global válida.

## Fuera de alcance

Configuración global (`~/.claude`, `~/.codex`, `~/.config/opencode`), instalación de
integraciones de Spec Kit y MCP: solo con autorización expresa.
