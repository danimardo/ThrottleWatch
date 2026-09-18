# Especificación visual y de experiencia

## Intención

ThrottleWatch debe sentirse como un instrumento técnico moderno: preciso, calmado y fácil de leer tanto en tema claro como oscuro. No debe copiar la cuadrícula densa de una herramienta de sensores tradicional. Las capturas de referencia aportan las variables —temperatura, TjMax, throttling por núcleo y relojes efectivos—, pero la interfaz las convierte en jerarquía visual y relato causal para una persona no experta.

La pregunta dominante de cada pantalla debe ser evidente:

- **Ahora:** “¿Estoy bien?”
- **Análisis:** “¿Qué ocurrió y por qué?”
- **CPU:** “¿Qué parte del procesador está afectada?”
- **Informe:** “¿Qué conclusión puedo defender?”

## Fuente canónica del sistema de diseño

El directorio `design/` es la fuente canónica para implementar la interfaz. Sus piezas se interpretan así:

- `design/tokens/tokens.css`: valores visuales y semánticos de color, tipografía, espaciado, forma, elevación y movimiento.
- `design/components/`: componentes Svelte reutilizables y sus contratos públicos.
- `design/examples/`: composición de referencia para pantallas, variantes y estados del producto.
- `design/AGENTS.md`: reglas de uso y extensión que debe seguir cualquier agente o desarrollador que trabaje dentro del sistema.
- `design/design-system.json`: inventario legible por herramientas y metadatos del sistema.
- `design/harness/`: entorno ejecutable para inspeccionar y validar los componentes sin depender de Tauri ni del hardware.

La aplicación DEBE consumir o trasladar estas piezas manteniendo una única fuente de verdad. Si hace falta un patrón que no existe, se amplía primero el sistema reutilizable; no se crea dentro de una pantalla una variante casi equivalente. Una desviación intencionada debe documentarse con su motivo, alcance y prueba visual. Esta sección determina la fuente de implementación; el resto del documento conserva los objetivos de experiencia y los criterios de aceptación.

## Principios de composición

1. **Una conclusión principal:** un único estado prominente, nunca una pared de números.
2. **Evidencia a un clic:** toda conclusión abre los sensores, ventanas y reglas que la originaron.
3. **Escalas honestas:** no truncar ejes para dramatizar; marcar límites y rangos normales.
4. **Movimiento con significado:** animar transición de estado, no cada cifra.
5. **Densidad progresiva:** resumen, análisis y sensores crudos en niveles separados.
6. **Color semántico estable:** el mismo color representa la misma clase en toda la app.

## Navegación

Barra lateral compacta:

- `Ahora`
- `Análisis`
- `CPU`
- `Sesiones`
- `Diagnóstico guiado`
- `Ajustes`

En ancho compacto (< 700 px) se convierte en `BottomBar` con exactamente cuatro ítems: `Ahora`, `Análisis`, `CPU` y `Más`; `Más` abre un menú con `Sesiones`, `Diagnóstico guiado` y `Ajustes`. `Informe` no está en la navegación: se abre desde `Ahora`, `Sesiones` o al terminar una prueba.

Mientras exista una prueba guiada activa, en **todos** los anchos y pantallas aparece bajo la barra de título una barra fija «Prueba en curso · fase · tiempo restante · Detener» (`Banner` de tono `warm` no descartable). Del mismo modo, el estado del colector cuando es `degraded`, `restarting`, `stopped` o `failed` se muestra como `Banner` global bajo la barra de título con las acciones «Reintentar» y «Ver resumen técnico».

## Primer inicio

El onboarding ocupa el contenido bajo la barra de título propia y consta exactamente de cinco diapositivas. Siempre muestra progreso (`1 de 5`), `Atrás`, `Siguiente` y `Omitir`; la última sustituye `Siguiente` por `Abrir Ahora`. La última diapositiva alcanzada se guarda al avanzar y se recupera tras un cierre inesperado.

1. **Bienvenida:** “Entiende si el calor está limitando tu CPU”, con una explicación de que ThrottleWatch observa y diagnostica, pero no modifica el equipo.
2. **Qué observa:** temperatura, carga, frecuencia activa y potencia mediante cuatro visuales simples. Los términos técnicos tienen una aclaración breve accesible por teclado; por ejemplo, *thermal throttling* es “la reducción automática de velocidad para evitar demasiado calor”.
3. **Qué puede concluir:** diferencia entre temperatura alta, limitación térmica, límite de potencia y datos insuficientes. Introduce el límite térmico (TjMax) y la frecuencia base («la velocidad que el fabricante garantiza») sin asumir conocimientos previos, explica que llegar al límite térmico puede ser normal si la frecuencia sigue por encima de la base, y evita prometer un porcentaje.
4. **Privacidad y decisiones:** funcionamiento local, ausencia de telemetría, historial de siete días, notificaciones apagadas y posibilidad de cambiar idioma, tema y segundo plano desde Ajustes.
5. **Este equipo:** comienza automáticamente la detección pasiva, muestra CPU, estado del colector y **nivel de cobertura** (A, B o C) con lo que permite concluir. Si el equipo no está en nivel A, explica en una frase qué se gana con el acceso avanzado («confirmar la causa y estimar cuánto ayudaría enfriar mejor»), lo presenta como recomendado y ofrece una acción explícita para instalarlo; nunca abre UAC por entrar en la diapositiva. `Abrir Ahora` es la acción principal y `Ejecutar diagnóstico guiado` la secundaria.

Omitir está disponible en las cuatro primeras diapositivas, no pide confirmación y conduce a `Ahora`; si aún no comenzó la detección, se inicia allí. Completar u omitir marca la versión actual como resuelta. Las novedades de una versión posterior usan tarjetas breves independientes y no repiten el recorrido completo. El recorrido completo puede abrirse otra vez desde `Ajustes → Acerca de y ayuda` sin alterar la marca de finalización.

## Barra de título y geometría

La ventana no usa decoraciones estándar de Tauri. Una barra propia, visible también durante onboarding y errores de arranque, contiene:

- icono y nombre `ThrottleWatch` a la izquierda;
- una región de arrastre que excluye todos los controles interactivos;
- doble clic en la región de arrastre para maximizar o restaurar;
- botones de minimizar, maximizar/restaurar y cerrar a la derecha;
- icono de maximizar que cambia a restaurar según el estado;
- foco visible, nombres accesibles y orden de teclado predecible;
- color de acento en reposo y tratamiento crítico del cierre solo en hover/foco, adaptado a los tokens de ThrottleWatch.

Se guardan posición, tamaño restaurado y estado maximizado al mover, redimensionar o cerrar. No se restaura el estado minimizado. Tamaño mínimo 480×600 px lógicos, reducido a la altura útil del monitor cuando no alcanza 600 (nunca por debajo de 480×500, caso típico de 1080p al 200 %); con altura útil inferior a 500, la ventana se maximiza y el contenido se desplaza verticalmente con `TitleBar`, `Banner` global y `Detener ahora` fijos; primer arranque 1100×760 centrado y ajustado al área útil. Antes de aplicar una posición se comprueba que una parte utilizable quede dentro de un monitor activo; si no, la ventana se centra en la pantalla principal respetando su tamaño mínimo.

La primera X sin preferencia guardada abre `FirstCloseDialog` (composición sobre `Dialog`) con `Salir de ThrottleWatch` y `Continuar en la bandeja y seguir midiendo`. Elegir bandeja activa la monitorización en segundo plano y el diálogo lo dice. La elección se guarda siempre y el diálogo explica que puede cambiarse en Ajustes; cerrarlo con `Esc` no guarda nada y la ventana sigue abierta. No se confunde minimizar con ocultar en bandeja.

Si al cerrar hay una operación en curso: con prueba activa, diálogo «Detener y salir / Cancelar»; con exportación, se espera hasta 5 s y se cancela; con descarga de actualización, se cancela y descarta; durante una instalación el cierre no está disponible.

## Pantalla “Ahora”

### Franja superior (`ContextStrip`)

Componente propio, siempre visible sobre el hero, en una línea (dos en compacto):

- Nombre del procesador y topología resumida (`8P + 16E`).
- Fuente de alimentación (`CA` / `Batería 64 %`) y plan energético cuando se conoce.
- Estado del colector con punto de color y frescura («hace 1 s»); con muestra de más de 5 s pasa a tono `warm` y texto «datos obsoletos»; desconectado pasa a `unknown`.
- Acción “Ver cobertura”, que abre `CoverageMatrix` como panel lateral (o diálogo a pantalla completa en compacto).

Cuando el colector está desconectado o la cobertura es insuficiente, el hero pasa a `indeterminate` y el `ContextStrip` explica por qué.

### Hero de estado

Componente central con tres capas:

1. **Anillo térmico**: temperatura actual respecto al **límite térmico efectivo** (TjMax − TCC offset); si no se conoce, escala explícitamente aproximada.
2. **Estado textual** con su gravedad cuando hay limitación:

| Clasificación | Gravedad `boost` (≥ frecuencia base) | Gravedad `below_base` |
|---|---|---|
| térmica confirmada / probable | «Limitada por temperatura · dentro de especificación» (tono `warm`) | «Throttling térmico · por debajo de la frecuencia garantizada» (tono `thermal`) |
| potencia | «Limitada por potencia · dentro de especificación» | «Limitada por potencia · por debajo de la frecuencia garantizada» |
| equipo · chasis | «Limitada por el calor del equipo (el fabricante reduce la potencia)» | ídem, tono `thermal` |
| equipo · señal externa | «Limitada por una señal externa (alimentación o batería)» | ídem |
| mixta | «Temperatura y potencia a la vez» | ídem, tono `thermal` |

Sin limitación: `Normal`, `Temperatura alta (sin pérdida demostrada)`, `Datos insuficientes`. Durante la ventana de turbo se añade la nota «turbo inicial en curso».

3. **Potencial con mejor refrigeración** (sustituye a «rendimiento disponible»): solo en nivel A y con limitación térmica, mixta o de chasis. Muestra el tramo y el rango redondeado a múltiplos de 5 %: «Enfriar mejor: +5–10 % (mejora moderada)». Sin límite de potencia conocido: «No cuantificable en este equipo» y, si la gravedad es `below_base`, «probablemente notable». En limitación de potencia: «La refrigeración apenas influye».

Ejemplo:

```text
THROTTLING TÉRMICO · POR DEBAJO DE LA FRECUENCIA GARANTIZADA
98 °C · límite 100 °C · frecuencia activa 2,1 GHz (base 2,6 GHz)
Enfriar mejor: +10–20 % (mejora notable)
Confianza alta · nivel A · observado durante 3 min 42 s
```

### Tarjetas de señal

Cuatro tarjetas en escritorio, dos columnas en ancho medio:

- Temperatura y margen.
- Carga y núcleos activos (P/E/LP).
- Frecuencia activa por grupo frente a su frecuencia base.
- Potencia del paquete y estado de límite.

Cada tarjeta contiene valor, mini-tendencia de 60 s, unidad, calidad y acceso al detalle. Una tarjeta sin sensor conserva su espacio y explica la ausencia.

### Narrativa causal

Un carril horizontal muestra, cuando existe evidencia:

```text
Carga sostenida → temperatura en el límite → potencia por debajo de su límite → frecuencia por debajo de la base
```

Si el diagnóstico es de potencia:

```text
Carga sostenida → potencia en su límite → temperatura con margen → frecuencia estable
```

Si es del equipo (chasis):

```text
Carga sostenida → el fabricante bajó el límite de potencia → CPU con margen → frecuencia menor
```

El fin del turbo nunca aparece como eslabón de una limitación; se muestra como marcador informativo en `Análisis`.

No mostrar el carril si los datos no sostienen la secuencia.

## Pantalla “Análisis”

### Gráfico principal sincronizado

Cuatro pistas verticales que comparten eje temporal y cursor:

1. Temperatura + línea de límite/margen.
2. Frecuencia activa por grupo P/E/LP o total homogéneo, con la frecuencia base como línea de referencia.
3. Carga por grupo y total.
4. Potencia con el límite de potencia efectivo como línea (si se conoce) + bandas de evento térmicas, eléctricas, del equipo y mixtas; el fin del turbo, como marcador informativo.

Interacciones:

- hover con valores del mismo instante;
- zoom y selección de rango;
- doble clic para volver a sesión completa;
- leyenda que oculta series sin reescalar de forma engañosa;
- selección de evento centra y abre evidencias;
- exportación del rango seleccionado.

Los huecos de muestreo se dibujan como huecos. Los puntos de calidad reducida se representan con trazo discontinuo y patrón, además de color.

### Panel de evidencias

Para el rango activo:

- Conclusión y confianza.
- Señales a favor.
- Señales que faltan.
- Causas alternativas detectadas.
- Nivel de cobertura, gravedad y, si hay potencial, sus entradas (límite de potencia usado, potencia medida, método).
- Regla/versiones del motor.

## Pantalla “CPU”

### Mapa de topología

Cuadrícula por grupos y núcleos:

- Tamaño constante para comparar.
- Relleno por temperatura o reloj, seleccionable.
- Borde/forma distingue P, E y LP; no solo color.
- Etiqueta corta (`P2`, `E6`) y valor actual.
- Tooltip accesible con temperatura, carga, frecuencia activa frente a base y razones de limitación observadas.

### Tabla avanzada

Vista opcional inspirada en la precisión de HWiNFO, no en su estética:

- Sensor normalizado.
- Nombre original.
- Actual, mínimo, máximo y promedio.
- Unidad, calidad y procedencia.
- Filtros por magnitud y grupo.

Esta tabla es secundaria y virtualizada; no aparece en el inicio.

## Panel “Cobertura” (`CoverageMatrix`)

Tabla de magnitudes × estado, reutilizada en `Ahora` (panel) y en `Ajustes → Sensores y cobertura` (embebida):

- Filas: temperatura, límite térmico efectivo, carga por núcleo, frecuencia activa, frecuencia base, potencia, límite de potencia, razón térmica, PROCHOT, razón de potencia, razón de corriente.
- Columnas: disponible (icono + texto), calidad (`directo` / `derivado` / `sustituto`), sensor de origen, motivo de ausencia.
- Cabecera: **nivel de cobertura** (A completo / B con potencia / C básico) con una frase de lo que permite concluir; en B y C, el acceso avanzado se presenta como recomendado para llegar al nivel A.
- Pie: «Confianza máxima alcanzable en este equipo: alta / media / baja» y estado del acceso avanzado (`not_needed | available | installable | denied | error`) con «Instalar/Reparar acceso avanzado» solo en `installable`.
- Acciones: «Volver a comprobar» y «Copiar resumen técnico».

## Pantalla “Informe”

Orden narrativo:

1. Resultado en una frase.
2. Qué se observó.
3. Qué impacto puede estimarse: potencial con mejor refrigeración (tramo y rango, con método y entradas) o, en sesiones guiadas, el rendimiento medido de la carga de prueba desglosado por causa. Nunca una cifra sin método.
4. Por qué se atribuye a temperatura/potencia/mezcla.
5. Qué no puede concluirse.
6. Recomendaciones ordenadas por coste y probabilidad.
7. Comparación antes/después si existe.

Recomendaciones térmicas posibles, nunca automáticas sin evidencia: revisar obstrucciones, base/superficie, perfil de ventilación, mantenimiento profesional, comparar con una nueva sesión. La aplicación no prescribe cambios peligrosos.

Acciones del informe: `Exportar` (abre `ExportDialog` con alcance «informe»), `Usar como referencia` / `Retirar referencia` (solo diagnósticos guiados completados; con confirmación) y, en sesiones importadas, `Re-evaluar con reglas actuales`, que muestra la evaluación nueva junto a la original sin sustituirla. Un informe de sesión activa se marca «Provisional · sesión en curso».

## Diagnóstico guiado

Pantalla de foco con:

- explicación previa estructurada («Qué va a pasar»: tipo de carga, duración total según `guided.duration`, sensores que se usarán, condiciones de parada automática, advertencia de que no es un benchmark);
- estado de sensores y alimentación (en batería: advertencia; con `guided.require_ac` activo: bloqueo explicado);
- botón principal `Iniciar diagnóstico`;
- seis pasos: Comprobación · Reposo (opcional, con «Omitir reposo») · Calentamiento · Carga sostenida · Recuperación · Resultado;
- progreso por fase con tiempo restante, temperatura grande, límite efectivo y margen, frecuencia activa frente a base y el rendimiento medido en vivo (operaciones/s);
- alcanzar el límite térmico no detiene la prueba; la interfaz lo explica («tu procesador se protege solo; es lo que queremos medir»);
- botón `Detener ahora` permanente, con atajo `Ctrl+Shift+X`; `Esc` no detiene;
- animación calmada, no gamificada;
- final con informe: rendimiento sostenido / inicial de la carga de prueba, desglose por causa (fin del turbo, potencia, temperatura, equipo), potencial con mejor refrigeración si hay nivel A, comparación con la referencia si existe, y acción «Usar como referencia».

Ocultar la ventana (bandeja) o una suspensión cancelan la prueba y lo explican en el resultado.

## Ajustes

La pantalla usa secciones con descripción corta y mantiene las opciones avanzadas plegadas inicialmente:

Cada sección puede tener un bloque «Avanzado» plegado por defecto (estado no persistido) con un `disclosure` propio; ningún ajuste avanzado aparece desplegado en el primer uso.

- **General:** acción al cerrar como `SegmentedControl` `Salir` / `Bandeja`; mientras no se haya decidido, ninguna opción marcada y texto «Se te preguntará al cerrar». Inicio con Windows desactivado por defecto; al activarlo, `Mostrar ventana` o `Iniciar oculto en la bandeja`, esta última disponible solo con monitorización de bandeja.
- **Idioma:** `Usar idioma del sistema`, `Español` y `English`. El idioma efectivo se muestra debajo; el cambio manual es inmediato y el cambio del sistema se recoge en el siguiente inicio.
- **Apariencia:** tema `Sistema`, `Claro` y `Oscuro`; movimiento `Sistema` / `Reducido` / `Completo` (sigue Windows por defecto); efecto de vidrio `Sistema` / `Completo` / `Reducido` / `Sin` (`Sistema` sigue «Efectos de transparencia» de Windows).
- **Monitorización:** perfiles `Bajo consumo`, `Normal` y `Diagnóstico`; `En batería`: `Mantener` / `Bajo consumo` / `Pausar`. Avanzado: intervalo exacto (solo lectura salvo en `Normal`), `Guardar detalle por núcleo en el historial`.
- **Bandeja y notificaciones:** monitorización en segundo plano; avisos apagados por defecto; `Probar notificación`. Avanzado: periodo de silencio (inicio/fin) y explicación de la persistencia mínima y el enfriamiento (solo lectura).
- **Datos y privacidad:** retención `Solo esta sesión`, `1 día`, `7 días` o `30 días`; espacio usado (base de datos, registros, sesiones, muestra más antigua); `Anonimizar exportaciones` (por defecto activo; solo afecta a ficheros); `Exportar…` (abre `ExportDialog` para la sesión activa o la última); `Eliminar todos mis datos`. No existe ninguna opción de envío de datos.
- **Sensores y cobertura:** `CoverageMatrix` embebida con estado del colector, «Volver a comprobar» y acción explícita de instalar/reparar acceso avanzado solo en estado `installable`.
- **Diagnóstico:** duración de la carga sostenida `Corta` (3 min) / `Estándar` (4 min) / `Larga` (6 min); `Exigir alimentación conectada` (apagado); `Avisar al terminar` (apagado; requiere notificaciones). Límites de seguridad visibles y en solo lectura (temperatura por encima del límite efectivo + 2 °C, frecuencia activa < 50 % de la base en el límite durante 10 s, sensor crítico perdido, generador sin respuesta; alcanzar el límite térmico no detiene la prueba), sin controles de voltaje, potencia o ventiladores.
- **Actualizaciones:** interruptor apagado por defecto; última comprobación; `Buscar actualizaciones`; versión disponible y notas; `Descargar` (solo en `available`) con progreso; `Instalar` (solo en `verified`, con motivo si está bloqueado). Sin selección de canal.
- **Acerca de y ayuda:** versión y build; `Repetir introducción`; `Documentación` (empaquetada) y `Documentación en línea` (abre el navegador del sistema, indicado); `Licencias de terceros` (abre `LicensesScreen`); `Resumen técnico` (`TechnicalSummary` con «Copiar»); `Abrir carpeta de registros`. Avanzado: interruptor `Registro detallado` (sube el registro a depuración; se desactiva solo a las 24 h o al reiniciar y muestra cuándo vence).
- **Zona de riesgo:** `Restablecer ThrottleWatch`, visual y textualmente distinto de borrar datos.

Los controles con dependencias no desaparecen: se muestran deshabilitados y explican la condición. Borrar datos conserva las preferencias; restablecer borra ambos y avisa que el siguiente arranque repetirá el primer inicio.

El interruptor general de notificaciones gobierna alertas térmicas y eléctricas. Activar voluntariamente las actualizaciones habilita su aviso nativo de versión disponible como parte de esa función independiente.

## Sesiones, exportación e importación

`Sesiones` lista sesiones pasivas, guiadas e importadas (estado `active | completed | cancelled | incomplete | imported`, con motivo en las incompletas), con marca «Referencia» cuando corresponda. La cabecera tiene `Importar…`, que abre el selector nativo y muestra el resultado (validación, migración, avisos) en un `Dialog`. Cada tarjeta ofrece abrir, exportar y eliminar; la sesión activa no puede eliminarse.

`ExportDialog` es único para los tres alcances (sesión, informe, rango de `Análisis`): elige formato (`CSV de muestras` / `JSON de informe`), `Anonimizar` (preseleccionado según preferencia) y muestra dos listas: campos incluidos y campos excluidos, junto con el tamaño estimado y el nombre de fichero propuesto. La ruta la elige el diálogo nativo del sistema; la interfaz nunca la muestra ni la recibe.

## Novedades tras una actualización

Tras una actualización con novedades pertinentes (cambios de permisos o comportamiento), `WhatsNewCards` muestra sobre `Ahora` una pila de tarjetas breves (título, una frase, acción opcional «Ir a Ajustes») con «Entendido». No repite el recorrido y no bloquea la detección.

## Idioma y contenido técnico

No hay cadenas visibles incrustadas en componentes. Español e inglés cubren interfaz, onboarding, accesibilidad, bandeja, notificaciones y errores. En modo `Sistema`, `es`, `ca`, `gl`, `eu`, `ast` y `an` se presentan en español; el resto, incluido `pt`, en inglés. `ca-AD` también se presenta en español.

Los textos deben admitir expansión inglesa/española sin truncar acciones ni depender de una longitud fija. CSV y JSON mantienen claves técnicas inglesas; los informes legibles se renderizan en el idioma efectivo. Números y fechas se formatean con `Intl` según el idioma efectivo (`es-ES`: `98,5 °C`, `16 sept 2026`; `en-US`: `98.5 °C`, `Sep 16, 2026`); la temperatura se muestra siempre en °C.

## Actualizaciones

La sección distingue visualmente tres estados y tres gestos: comprobar, descargar e instalar. Activar el interruptor habilita comprobaciones automáticas, pero no descarga. `Buscar actualizaciones` es una comprobación manual incluso si no han pasado 24 horas. Una versión disponible aparece también como notificación de Windows; abrirla lleva a Ajustes.

La descarga muestra bytes/progreso cuando se conocen y no ofrece `Instalar` hasta verificar la firma. Instalar abre una confirmación que anuncia el cierre. Si existe un diagnóstico o exportación en curso, el botón permanece bloqueado con explicación. Los errores de red o firma se contienen dentro de esta sección y nunca sustituyen el estado térmico principal.

## Sistema visual

### Temas claro y oscuro

| Token | Valor inicial | Uso |
|---|---:|---|
| `surface-0` | `#090D14` | fondo |
| `surface-1` | `#101722` | panel |
| `surface-2` | `#172131` | elevación |
| `text-1` | `#F3F7FC` | texto principal |
| `text-2` | `#AAB7C8` | texto secundario |
| `accent` | `#59D8FF` | selección/datos neutros |
| `normal` | `#55D6A5` | normal |
| `warm` | `#FFC857` | advertencia |
| `thermal` | `#FF5D73` | térmico |
| `power` | `#A58BFF` | potencia |
| `unknown` | `#8793A3` | desconocido |

La tabla define el punto de partida oscuro; cada token tiene un equivalente claro aprobado. `Sistema` escucha el tema de Windows en caliente; una selección manual prevalece. `thermal` y `power` siempre se acompañan de icono/patrón/etiqueta.

### Tipografía

- Interfaz: tipografía variable sans legible y redistribuible, preferentemente del sistema.
- Datos: variante tabular/monoespaciada para evitar saltos de ancho.
- Números grandes sin más de una cifra decimal salvo vista avanzada.

### Interacción nativa, no de página web (enmienda 2026-09-18)

ThrottleWatch DEBE sentirse como una aplicación de escritorio de Windows, nunca como una pestaña de navegador servida dentro de la ventana:

- **Ningún subrayado aparece al pasar el puntero o al recibir foco.** Ni en botones, ni en filas con estilo de enlace (por ejemplo, un `OptionRow` con chevron, «Ver informe», «Ver cobertura», el enlace «Documentación en línea»), ni en pestañas o chips. El estado de hover/activo/foco se comunica con color, peso tipográfico y el anillo de foco ya definido; nunca con un `text-decoration: underline` que aparece o desaparece.
- **Ningún control usa el cursor de mano.** Botones, pestañas, chips, controles segmentados y filas con chevron usan el cursor de flecha estándar (`cursor: default`), igual que los controles nativos de Windows. La única excepción documentada es la superficie de trazado de `AnalysisChart` (cursor de mira al desplazar el cursor temporal, cursor de mano sobre una banda de evento pulsable): ahí el cursor comunica una interacción gráfica de datos, no un enlace.
- **La navegación y las acciones dentro de la aplicación usan controles de botón, nunca hiperenlaces de navegador.** Un enlace real (`<a href>`) solo se reserva para abrir algo fuera de la aplicación (por ejemplo, `Documentación en línea`, que abre el navegador del sistema) y, aun así, se le aplican las mismas dos reglas anteriores: sin subrayado y sin cursor de mano.

### Material de vidrio (enmienda 2026-09-18)

Todas las superficies flotantes y las tarjetas de contenido usan un material de vidrio inspirado en el lenguaje «Liquid Glass» de Apple, adaptado a Windows y a la legibilidad de un instrumento:

- **Tres intensidades**: `fuerte` para el chrome flotante (barra de título, barra inferior y su menú, diálogos, tooltips, listas desplegables, paneles laterales), `base` para tarjetas de contenido (hero, tarjetas de señal, carril causal, secciones de Ajustes, informe, sesiones, cobertura, paneles del diagnóstico) y `sutil` para tiles y controles que ya están sobre otro vidrio (botón secundario, control segmentado, núcleos del mapa).
- **Opacidad alta a propósito** (80 % oscuro / 76 % claro en tarjetas; 90 % / 88 % en chrome): el vidrio deja intuir el fondo, nunca compite con el texto. El contraste AA se verifica sobre el fondo ambiental más claro y más oscuro posibles.
- **Composición del material**: desenfoque 18 px con saturación 1,35, borde luminoso de 1 px, brillo especular diagonal muy suave, y una única sombra difusa de elevación. No se apilan más de dos capas de desenfoque en la misma región de pantalla.
- **Fondo ambiental**: el shell dibuja bajo todo un fondo con tres manchas de color (acento, potencia y térmico a baja opacidad) que derivan muy lentamente (48 s). Es lo que el vidrio refracta; no transmite información.
- **Niveles de calidad**: `completo`, `reducido` (sin desenfoque, opacidad 92–96 %) y `sin` (superficies sólidas con línea fina). El valor inicial `Sistema` sigue «Efectos de transparencia» de Windows; la aplicación baja el nivel automáticamente si el presupuesto de fotogramas de WebView2 se resiente (véase `plan.md`). Sin soporte de `backdrop-filter`, las superficies pasan a 94–97 % de opacidad.
- **Lo que el vidrio no toca**: los colores de estado y sus iconos, el anillo térmico, las bandas de eventos y los patrones de calidad reducida conservan su semántica y su contraste; el vidrio es el soporte, no el mensaje.

### Espaciado y forma

- Grid base de 4 px; separaciones principales 16/24/32 px.
- Radio de 12–18 px en tarjetas; bordes luminosos de 1 px y una sola sombra difusa (elevación del vidrio).

## Animación (enmienda 2026-09-18)

Principio: **movimiento en las transiciones y los cambios de estado; calma en reposo.** Nada pulsa continuamente para un estado normal; la única excepción es el halo del anillo térmico, que respira con un ciclo de 6 s y una variación de opacidad tan baja que no distrae.

Tokens: `rápido` 160 ms, `base` 240 ms, `lento` 420 ms; curvas `ease-out`, `ease-in-out` y `spring` (con ligero rebote); escalonado de 45 ms entre elementos hermanos.

Catálogo de animaciones:

- **Entrada de pantalla**: el contenido entra con desplazamiento de 14 px y fundido (320 ms); la pantalla saliente se funde en 120 ms.
- **Entrada escalonada**: tarjetas de señal, filas de cobertura, tarjetas de novedades, sesiones, bloques del informe y secciones de Ajustes aparecen con elevación y fundido, escalonadas 45 ms.
- **Anillo térmico**: el arco interpola su longitud (420 ms) y su color al cambiar de clasificación; la cifra central hace un fundido ascendente al cambiar; la etiqueta de clasificación aparece con «pop» elástico solo cuando cambia la clasificación.
- **Cambio de valor**: la cifra de una tarjeta de señal hace un pequeño rebote de escala (1,06) al cambiar; el texto siempre es el valor real.
- **Barra de progreso**: el relleno se anima con curva elástica y recibe un barrido especular de una sola iteración al cambiar.
- **Chrome flotante**: el menú «Más» emerge desde su esquina con rebote; los diálogos aparecen con escala 0,92 → 1 y el fondo se desenfoca; tooltips y listas desplegables usan la misma aparición breve.
- **Pulsación líquida**: botones, segmentos y elementos de navegación se contraen al 97 % al pulsar y los botones emiten una onda radial desde el punto de contacto.
- **Evidencia**: las bandas de evento del gráfico se despliegan verticalmente al aparecer; el carril causal dibuja sus conectores de izquierda a derecha tras cada nodo.
- **Colector**: al reconectar, el punto de la franja de contexto emite un único anillo expansivo; mientras arranca, late.
- **Onboarding**: cada diapositiva hace «pop» de su ilustración y fundido ascendente del texto.
- **Hover**: las tarjetas se elevan 2 px y ganan sombra; los núcleos del mapa se elevan 1 px.

Movimiento reducido (`prefers-reduced-motion` o `Movimiento: Reducido`): todas las animaciones y transiciones colapsan a un cambio instantáneo; solo sobreviven fundidos imperceptibles. `Movimiento: Completo` permite anular la preferencia del sistema de forma explícita.

## Estados esenciales

Cada pantalla debe diseñarse para:

- cargando/detectando;
- compatible completo;
- cobertura parcial;
- controlador ausente;
- sidecar desconectado;
- sin historial;
- sesión activa;
- datos obsoletos;
- error recuperable;
- error que requiere intervención.

## Accesibilidad

- Orden de foco coincidente con jerarquía visual.
- Atajos: `Ctrl+1`…`Ctrl+6` navegación (Ahora, Análisis, CPU, Sesiones, Diagnóstico guiado, Ajustes); `Ctrl+,` Ajustes; `Ctrl+Shift+X` detener prueba; `Ctrl+E` exportar; `F1` ayuda; `Esc` cierra diálogos y tooltips. `Esc` nunca detiene una prueba.
- Gráficos con resumen textual, tabla de datos del rango y nombres accesibles.
- Tooltips accesibles por teclado y que no desaparecen al mover el puntero hacia ellos.
- Escalado de Windows hasta 200 % sin ocultar la parada de la prueba.
- Contraste AA mínimo; objetivos AAA para texto principal cuando sea viable.

## Presupuesto visual y técnico

- Máximo recomendado de 2.000–3.000 puntos por pista y 10.000–12.000 puntos totales; Rust agrega rangos largos según resolución y el zoom solicita una ventana más detallada.
- La agregación conserva picos, huecos, calidad material y límites de eventos; nunca une visualmente un intervalo sin muestras.
- Mapa de núcleos actualiza a 1 Hz, no a frecuencia de animación.
- No más de dos capas de desenfoque superpuestas en la misma región; el nivel de vidrio baja a «reducido» automáticamente si la tasa de fotogramas cae por debajo del presupuesto.
- La UI debe seguir interactiva mientras se importan/exportan sesiones.
- Cualquier degradación de gráficos debe reducir detalle, no bloquear la adquisición.

## Criterios de aceptación visual

- En cinco segundos un usuario distingue estado y causa principal.
- Temperatura alta y pérdida demostrada no se confunden visualmente.
- El porcentaje de tiempo en throttling aparece rotulado como tiempo, nunca junto a la barra de rendimiento sin aclaración.
- Un usuario puede rastrear una conclusión hasta las muestras que la sustentan.
- Capturas a 100/125/150/200 % no presentan texto cortado ni controles inaccesibles.
- Tema claro/oscuro y movimiento reducido superan pruebas visuales automatizadas.
