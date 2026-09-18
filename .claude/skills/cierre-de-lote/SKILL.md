---
name: cierre-de-lote
description: Cierra un lote funcional Lxx de tasks.md de ThrottleWatch. Úsala cuando la implementación del lote esté completa y haya que ejecutar su suite, verificar el checkpoint CHK-Lxx y registrar el resultado. No la uses para cerrar tareas sueltas ni historias enteras.
---

<!-- GENERADO por scripts/agent/sync-adapters.mjs a partir de .agents/skills/cierre-de-lote/SKILL.md. No editar: cambia la fuente y vuelve a sincronizar. -->

# Cierre de lote

Entrada: identificador del lote (`L00`…`L21`). Fuente: `specs/001-cpu-thermal-diagnostics/tasks.md`
y `historias.md` § 35 («Definition of Checkpoint»).

## Pasos

1. **Localiza el lote** en `tasks.md`: sus tareas, su plan de pruebas (bajo el encabezado del lote)
   y su tarea `CHK-Lxx`. Si el plan de pruebas no existe, detente y pídelo: no se cierra un lote sin él.
2. **Comprueba la implementación:** todas las tareas del lote marcadas `[x]` salvo el `CHK`. Si
   falta alguna, lista cuáles y para.
3. **Nivel 0 de las pilas tocadas** (constitución XVI), en este orden y parando al primer fallo:
   formato → lint → `pnpm check` (o `cargo clippy -D warnings` / analizadores .NET) → compilación.
   Usa solo comandos que existan en `package.json`, `Cargo.toml` o `*.csproj`.
4. **Suite del lote:** ejecuta únicamente lo que indica el plan del lote (unitarias, componentes,
   integración, aceptación o corpus afectados). No ejecutes la matriz completa ni los E2E salvo
   que el plan del lote lo pida.
5. **Deuda del lote:** confirma que no quedan tests pendientes, ignorados o `todo` sin
   justificación ni incidencia; que existen los fixtures y fakes que necesitan los lotes
   siguientes; y que `traceability.md` (si existe) recoge los requisitos del lote.
6. **Revisión frente a la constitución:** ejecuta el rol `revisor-constitucion` (subagente en
   Claude/Codex) sobre el diff del lote, o repasa a mano las puertas de calidad aplicables.
7. **Registra el resultado** en la tarea `CHK-Lxx` de `tasks.md`: orden ejecutada, código de
   salida, número de errores y avisos, fecha. Marca `CHK-Lxx` como `[x]` solo si todo es cero.
8. **Informa** a la persona usuaria con: qué pasó, qué no pudo ejecutarse y por qué, y cuál es el
   siguiente lote según «Orden de lotes» de `tasks.md`.

## Prohibido

- Declarar superada una comprobación que no se ejecutó.
- Subir `timeouts`, añadir reintentos o `skip` para que pase la suite.
- Hacer commit o push: eso lo pide la persona usuaria aparte.
