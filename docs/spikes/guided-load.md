# Spike T053: carga guiada segura

**Estado:** resuelto con observación externa
**Fecha:** 2026-09-19

La primera implementación no inicia una carga sintética dentro del proceso de
ThrottleWatch. La aplicación observa una carga externa que el usuario inicia
fuera de la ventana y ejecuta la misma secuencia de adquisición, seguridad y
cancelación. Esto mantiene el diagnóstico útil sin crear una superficie que
pueda elevar la temperatura o el consumo de forma inesperada.

## Comparativa

| Criterio | Carga integrada | Observación externa |
|---|---|---|
| Trabajo por hilo y segundo | Medible | No disponible |
| Parada segura | Requiere controlar el generador | Observa pérdida de sensor, exceso térmico y suspensión |
| Permisos | Ninguno adicional, pero aumenta riesgo operativo | Ninguno adicional |
| Reproducibilidad | Depende del generador incluido | Depende de la carga elegida por la persona |
| Antes/después | Posible | Diferida |

## Decisión experimental

La ruta externa es la aprobada para la primera versión. El resultado guiado
expone clasificación, gravedad, reloj activo y observaciones, todos rotulados
como observación. No serializa porcentaje de rendimiento, potencial calculado ni
comparaciones antes/después. La carga integrada queda bloqueada hasta disponer
de un generador revisado, límites de seguridad instrumentados y una segunda
revisión de seguridad.

## Procedimiento reproducible

1. La interfaz muestra qué sensores se observarán y las condiciones de parada.
2. La persona inicia una carga externa conocida y pulsa `Iniciar diagnóstico`.
3. El colector captura reposo, calentamiento, carga y recuperación con el perfil
   elegido; alcanzar el límite térmico no detiene la observación.
4. Se cancela por exceso de límite efectivo, reloj inferior al 50 % de la base
   durante 10 s, tres muestras sin sensor crítico, falta de muestras del
   colector o suspensión.
5. El informe declara explícitamente que no es un benchmark y que no contiene
   una cifra de rendimiento.
