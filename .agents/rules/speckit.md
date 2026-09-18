---
paths:
  - "specs/**"
  - ".specify/**"
  - "historias.md"
---

# Reglas para documentos de Spec Kit

- Los ficheros de `specs/<feature>/` se editan solo a través de las skills `speckit-*` o con
  ediciones dirigidas que conserven su estructura; no se regeneran desde plantilla si ya tienen
  contenido revisado.
- `spec.md`: todo número que decida algo (umbral, ventana, duración) va en «Parámetros iniciales»
  con identificador; los requisitos usan `FR-`, `NFR-`, `SC-` y nunca se renumeran los existentes.
- `plan.md`: toda excepción a la constitución se registra en «Excepciones registradas» con motivo,
  alternativa, riesgo y fecha de retirada.
- `tasks.md`: las tareas conservan su identificador; las nuevas usan `T1xx` o `T-XXX-nnn`; cada
  lote termina con `CHK-Lxx`; las tareas de test van dentro de su lote.
- `historias.md`: las 311 líneas originales (hasta «Mapa hacia las especificaciones») no se
  modifican; solo se amplía a partir de «Estrategia integral de testing».
- Conserva los finales de línea del fichero (CRLF en `historias.md` y `specs/**`).
- `.specify/memory/constitution.md` requiere autorización expresa; si se enmienda, se incrementa
  la versión (MAYOR/MENOR/PARCHE) y se añade una entrada al historial con los documentos afectados.
- Después de tocar spec, plan o tasks, ejecuta `speckit-analyze` y corrige lo CRÍTICO antes de
  implementar.
