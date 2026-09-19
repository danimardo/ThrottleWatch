# ADR-0003: modo seguro de carga guiada

**Estado:** aceptado
**Fecha:** 2026-09-19

## Contexto

T053 comparó una carga integrada con la observación de una carga externa. Una
carga integrada permitiría medir operaciones por segundo, pero introduce una
acción activa sobre temperatura, potencia y estabilidad del equipo. La
constitución prioriza seguridad y veracidad sobre alcance.

## Decisión

ThrottleWatch adopta observación externa en el MVP. La prueba guiada puede
orquestar fases y paradas, pero no genera trabajo de CPU. El informe guiado
conserva clasificación, gravedad, relojes y evidencia, marcados como
observación. No ofrece porcentaje de rendimiento, potencial de refrigeración ni
comparación antes/después.

## Consecuencias

- Se cumple la degradación de FR-083 sin inventar una cifra de rendimiento.
- T049 y T051 deben persistir y mostrar un `guided_result` observacional sin
  método de rendimiento.
- US3-4, US3-5 y la parte guiada de SC-002 quedan diferidas.
- Cualquier futuro generador integrado necesitará un ADR de sustitución,
  pruebas de parada y una revisión de seguridad independiente.

## Alternativas descartadas

La carga integrada queda descartada para el MVP por no existir todavía un
generador acotado y verificable que permita demostrar parada segura bajo todas
las combinaciones de sensores, batería, suspensión y límites térmicos.
