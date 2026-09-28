# Prompt: principio de registro (logging) y diagnóstico

Prompt reutilizable para añadir o revisar el principio de registro de `.specify/memory/constitution.md`
en un proyecto Spec Kit. Nace del 2026-09-28, cuando ThrottleWatch tenía un principio de registro
completo en papel (XVII) que el código no cumplía: lints declarados pero no configurados, mensajes
censurados a `"[redacted]"` sin que fallara ningún test y subsistemas enteros sin una sola línea de
registro. Las secciones B, G y H existen para cerrar exactamente esos huecos.

Si el proyecto necesita imponer tecnologías concretas (por ejemplo Pino y `LOG_LEVEL` en un servidor
Node), añádelas en la sección opcional del final; si no, deja que se deduzcan de la pila real.

```text
Trabajamos con Spec Kit (https://github.com/github/spec-kit). En .specify/memory/constitution.md
están los principios innegociables del proyecto. Quiero añadir (o revisar, si ya existe) un
principio de REGISTRO (logging) Y DIAGNÓSTICO.

ANTES DE ESCRIBIR NADA:
1. Lee la constitución, el plan y el código para saber la pila real (lenguajes, capas, procesos,
   dónde se escriben hoy los registros). No supongas tecnologías: elige una biblioteca de registro
   por capa que encaje con esa pila y justifícala. Si ya hay un principio de registro, compáralo
   con el código real y dime qué reglas no se cumplen hoy.
2. Hazme las preguntas que necesites (por ejemplo: ¿hay servidor o es de escritorio?, ¿se envían
   registros fuera del equipo?, ¿qué datos personales maneja la app?, ¿qué ruido es aceptable en
   producción?). Espera mis respuestas y, después, propón el texto.

OBJETIVO DEL PRINCIPIO (el criterio que manda sobre los demás):
Con solo el fichero de registro de una ejecución, alguien (persona o agente de IA) que no vio lo
que pasó DEBE poder saber qué hizo la aplicación, qué subsistemas funcionaron, cuáles fallaron o
pasaron a modo degradado, y por qué, sin preguntar a la persona usuaria.

EL PRINCIPIO DEBE CUBRIR:

A. API única por capa
   - Un envoltorio (wrapper) por capa, que es la única API de registro permitida. Las
     funcionalidades no importan la biblioteca ni usan la salida estándar o de error.
   - Cada evento lleva un código estable (en inglés, MAYÚSCULAS_CON_GUIONES_BAJOS), un mensaje y
     campos estructurados.

B. Qué hay que registrar (cobertura obligatoria, no opcional)
   - Cada final de un proceso o máquina de estados (completado, cancelado, fallido), con el motivo.
   - Cada cambio de fase o de estado relevante (a nivel debug).
   - Cada caída a modo degradado o a una alternativa (por ejemplo, «no se pudo X, se usa Y»),
     con la causa original.
   - Cada error que se trague o convierta en un valor por defecto (sin error silenciado).
   - Arranque: versión, configuración efectiva (sin secretos) y estado de cada dependencia externa.
   - Toda funcionalidad nueva incluye en sus tareas la lista de eventos que emite.

C. Niveles y configuración
   - error/warn/info/debug/trace con significado común y ejemplos para ESTE proyecto.
   - Producción: nivel mínimo y ruido acotado (define un objetivo medible).
   - Desarrollo: debug por defecto, ajustable, y el ajuste se conserva entre ejecuciones.
   - Todo modo de depuración temporal en producción: define su caducidad y si sobrevive a reinicios.
   - Di explícitamente qué partes NO se pueden configurar en producción.

D. Formato
   - Fichero orientado a máquina (p. ej. JSON por líneas, marcas de tiempo UTC ISO 8601) con un
     esquema versionado.
   - Salida para personas en formato local (dd/MM/yyyy HH:mm:ss, zona Europe/Madrid, con la
     abreviatura CET/CEST); nunca marcas Unix en bruto.
   - Correlación: un identificador de sesión u operación que una los eventos de un mismo flujo
     entre capas.

E. Privacidad
   - Lista de datos prohibidos (secretos, tokens, identificadores personales o del equipo,
     rutas con el nombre del usuario…).
   - La redacción usa una lista permitida y NUNCA puede censurar el mensaje ni el código del evento:
     si un mensaje puede contener datos sensibles, se construye sin ellos, no se censura después.

F. Rendimiento
   - Escritura no bloqueante, rotación, agrupación de repeticiones, y el retraso máximo hasta que
     un evento llega al disco (documéntalo).

G. Verificación y cumplimiento (lo que suele faltar; es obligatorio)
   Para CADA regla «DEBE/NO DEBE» indica el mecanismo automático que la hace cumplir (lint,
   configuración de lint, test o paso de CI) y exige:
   - un test que demuestre que el mecanismo falla cuando se viola la regla (p. ej. código con
     console.log / println / uso directo de la biblioteca → el lint falla);
   - un test de extremo a extremo por capa: se emite un evento real a través del envoltorio y del
     subscriber o transporte real, se lee el fichero y se comprueba que código, mensaje y campos
     permitidos llegan íntegros y legibles, y que los datos prohibidos no aparecen;
   - un escenario de aceptación de diagnóstico por cada funcionalidad crítica: se provoca un fallo
     conocido y el test afirma que el log contiene el evento que identifica la causa;
   - que se compruebe, en la puerta de calidad, que las reglas declaradas existen de verdad en la
     configuración (p. ej. que la lista de macros prohibidas del lint incluye lo que dice la
     constitución).
   Si una regla no tiene un mecanismo automático posible, márcala como «revisión manual» y di
   quién la revisa y cuándo.

H. Uso por agentes y personas
   - Dónde están los registros, cómo se abren desde la app y cómo se activa el modo detallado.
   - Regla para agentes de IA: cuando la persona describa un fallo, leer primero el registro de
     esa ejecución y contrastarlo con su relato antes de proponer una causa.

FORMA DE LA RESPUESTA:
1. Diagnóstico: diferencias entre lo que diga hoy la constitución (si existe) y el código real.
2. Preguntas pendientes.
3. Tras mis respuestas: el texto del principio (con DEBE/NO DEBE), la subida de versión y la
   entrada del historial, una tabla «regla → mecanismo de cumplimiento → test que lo prueba» y las
   tareas necesarias para que el código cumpla (incluida la migración de lo que hoy no cumple).
No des por hecho que algo se cumple sin haberlo comprobado en el código o ejecutado.
```

## Sección opcional: decisiones tecnológicas fijadas

Añádela al final del prompt solo si el proyecto ya ha decidido las bibliotecas y no quieres que la IA
las elija. Ejemplo para un proyecto web con servidor Node:

```text
DECISIONES TECNOLÓGICAS FIJADAS (no las cambies; adapta el resto del principio a ellas):
- Servidor: Pino, nivel controlado por LOG_LEVEL.
- Cliente: loglevel, nivel controlado por PUBLIC_LOG_LEVEL.
- Un módulo envoltorio compartido es la única API de registro del código de aplicación.
```
