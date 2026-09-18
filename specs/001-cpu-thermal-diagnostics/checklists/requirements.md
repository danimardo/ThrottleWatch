# Lista de calidad de requisitos

**Propósito**: comprobar que la especificación está lista antes de implementar.  
**Fecha**: 2026-09-17  
**Feature**: `../spec.md`

## Calidad del contenido

- [x] `spec.md` describe qué y por qué sin imponer la arquitectura.
- [x] `plan.md` concentra las decisiones de cómo construirlo.
- [x] Las historias están priorizadas y son demostrables de forma independiente.
- [x] Los requisitos usan lenguaje verificable y evitan términos vagos sin criterio.
- [x] Los resultados medibles no dependen de una marca concreta de CPU.
- [x] Alcance y fuera de alcance están explícitos.

## Corrección del dominio

- [x] Temperatura alta no equivale automáticamente a throttling.
- [x] Tiempo con throttling no equivale a rendimiento perdido.
- [x] El turbo máximo no se usa como baseline multinúcleo sostenido.
- [x] Los núcleos híbridos se agrupan y no se promedian sin ponderación.
- [x] La ausencia de sensor no se convierte en cero.
- [x] Se consideran potencia, corriente, plan energético y batería.
- [x] El porcentaje solo existe con referencia local comparable.
- [x] La incertidumbre se comunica como rango/confianza.

## Cobertura de escenarios

- [x] Camino principal y estados degradados.
- [x] Procesadores Intel, AMD, híbridos y antiguos.
- [x] Sidecar/controlador ausente o fallido.
- [x] Suspensión/reanudación y cambios de energía.
- [x] Monitorización pasiva y prueba guiada opcional.
- [x] Retención, exportación, importación y anonimización.
- [x] Accesibilidad, rendimiento y seguridad.
- [x] Primer inicio, reanudación, omisión y cobertura parcial.
- [x] Español/inglés, resolución especial de locales y fallback.
- [x] Tema sistema/claro/oscuro, barra propia y restauración de ventana.
- [x] Ajustes, primera acción de cierre, borrado y restablecimiento.
- [x] Actualizaciones opt-in, firma, gestos separados y operaciones bloqueantes.
- [x] Coherencia con el sistema de diseño canónico, estados esenciales y matriz de ambos idiomas, ambos temas y tamaños compacto/medio/expandido.

## Preparación

- [x] No quedan marcadores `[NEEDS CLARIFICATION]` que bloqueen el MVP.
- [x] Los riesgos que exigen experimentación están aislados como spikes.
- [x] Existen contratos de protocolo y esquema inicial.
- [x] Existe modelo de datos y estrategia de migración.
- [x] Las tareas enlazan archivos e historias.
- [x] La fuente canónica de tokens, componentes, ejemplos y harness está identificada y tiene una puerta de verificación reproducible.

## Revisión de huecos (2026-09-18)

- [x] Contradicciones mockup ↔ spec resueltas y registradas en `../gap-analysis.md` §10.
- [x] Parámetros iniciales del ruleset v1, confianza, perfiles, alertas, prueba guiada y formatos fijados en `spec.md`.
- [x] Contrato de comandos completado (telemetría, cobertura, sesiones, exportación/importación, prueba guiada, bandeja, diagnóstico técnico).
- [x] Schema IPC ampliado a todos los tipos de mensaje.
- [x] Componentes de diseño pendientes registrados como T118–T129 y en `design/README.md`.
- [x] `/speckit-analyze` ejecutado (2026-09-18): críticos y altos corregidos (`../gap-analysis.md` § 13); quedan 11 medios y 11 bajos.

## Revisión del motor (2026-09-18)

- [x] Ninguna regla confunde el fin del turbo con limitación térmica (ventana de turbo, FR-077).
- [x] La causa se atribuye por mesetas o razones directas, nunca solo por una caída de frecuencia (FR-078).
- [x] La gestión térmica del fabricante tiene clase propia y no desaconseja ventilar (FR-079).
- [x] La gravedad se mide frente a la frecuencia garantizada (FR-080).
- [x] Ninguna cifra de rendimiento sin método físico explícito o medición guiada (FR-013, constitución III).
- [x] La prueba guiada puede observar la limitación térmica (FR-085).
- [x] Los criterios de acierto tienen verdad de referencia objetiva (corpus etiquetado, `research.md` § 15).
- [ ] Pendiente: puerta de viabilidad del nivel A (T019a) antes de la fase 3.

## Notas

- El nombre definitivo es `ThrottleWatch`.
- ARM64 no se promete hasta validar toda la cadena de dependencias.
- La carga integrada permanece condicionada al spike de seguridad; el modo de observación externa mantiene viable la historia si no se aprueba.
- El endpoint concreto de GitHub Releases se fija al crear el repositorio remoto; no es configurable en tiempo de ejecución.
