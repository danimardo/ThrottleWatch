# Especificación de funcionalidad: diagnóstico térmico de CPU

**Rama**: `001-cpu-thermal-diagnostics`  
**Creada**: 2026-09-17  
**Estado**: Planificada y con tareas (L00 pendiente)  
**Entrada**: Utilidad Windows visualmente atractiva que mida CPU Intel/AMD, detecte limitación térmica y explique cuánto rendimiento potencial se pierde sin confundir calor con otras limitaciones.

## Objetivo

ThrottleWatch ayuda a responder tres preguntas:

1. ¿Mi CPU está demasiado caliente en este momento o durante una carga sostenida?
2. ¿El calor está reduciendo realmente su rendimiento?
3. ¿Mejorar la refrigeración probablemente ayudaría, o el límite principal es otro?

La utilidad debe convertir sensores técnicos en una explicación verificable, conservar el detalle para usuarios avanzados y abstenerse de cuantificar cuando no exista evidencia suficiente.

## Aclaraciones

### Sesión 2026-09-19

- Q: Cuando el usuario pulsa la acción explícita «instalar acceso avanzado», ¿quién instala PawnIO? → A: ThrottleWatch empaqueta el instalador oficial de PawnIO (versión fijada, firma verificada) y lo ejecuta desde la acción explícita; una sola petición UAC.
- Q: Si un proceso de usuario estándar no puede abrir el dispositivo PawnIO, ¿cómo alcanza ThrottleWatch el nivel A sin elevar la interfaz diaria? → A: En la misma instalación por máquina se registra un lanzador elevado solo para el sidecar (tarea programada o servicio mínimo, a decidir en el plan); la interfaz sigue sin privilegios; la decisión se documenta en un ADR con revisión de amenazas.
- Q: Si ya existe un PawnIO instalado por otra aplicación, ¿ThrottleWatch lo reutiliza o instala el suyo? → A: Lo reutiliza si su versión es igual o superior a la mínima probada; si es anterior, ofrece actualizar por acción explícita; desinstalar ThrottleWatch nunca elimina PawnIO.
- Q: Cuando el acceso avanzado deja de estar operativo en mitad de una sesión, ¿qué hace ThrottleWatch con la sesión en curso? → A: Continúa la sesión degradando a nivel B/C; registra el cambio de nivel con marca de tiempo; evalúa cada ventana con el nivel vigente; aviso no intrusivo en cobertura con opción de reparar; sin alerta `collector_lost`.
- Q: Si durante una prueba guiada el acceso avanzado deja de funcionar (nivel A → B/C a mitad), ¿qué hace ThrottleWatch con la prueba y su resultado? → A: La prueba continúa hasta el final; el informe guiado registra el instante del cambio y el nivel final; la cifra de rendimiento y «marcar como referencia» solo se ofrecen si el nivel A se mantuvo durante toda la prueba; si no, el informe lo explica.

## Escenarios de usuario y pruebas

### Historia 1 — Comprender el estado térmico actual (Prioridad P1)

Como usuario quiero abrir la aplicación y entender en pocos segundos la temperatura, carga, frecuencia efectiva y estado térmico de mi CPU.

**Por qué es prioritaria:** es el valor mínimo del producto y permite validar la cobertura de sensores antes de inferir causas.

**Prueba independiente:** con una fuente de sensores compatible, el panel muestra datos actuales, tendencia y estado; con sensores incompletos, muestra claramente la cobertura reducida.

**Escenarios de aceptación:**

1. **Dado** un equipo con sensores de CPU disponibles, **cuando** se abre el panel, **entonces** aparecen modelo, temperatura representativa, carga, frecuencia activa (o sustituto marcado), potencia cuando exista y antigüedad de la última muestra.
2. **Dado** un sensor ausente, **cuando** se muestra su tarjeta, **entonces** figura como “No disponible” con explicación y nunca como `0`.
3. **Dado** que la temperatura se acerca al límite conocido, **cuando** se actualiza la muestra, **entonces** el panel muestra la distancia al límite y cambia de estado usando color, icono y texto.
4. **Dado** un procesador híbrido, **cuando** existen grupos P/E/LP, **entonces** la aplicación no mezcla sus relojes en una media sin ponderar ni ocultar la topología.

---

### Historia 2 — Detectar y explicar una limitación (Prioridad P1)

Como usuario quiero saber si la CPU reduce su capacidad por temperatura, potencia u otra causa para tomar una decisión correcta.

**Por qué es prioritaria:** diferencia el producto de un simple termómetro.

**Prueba independiente:** al reproducir trazas etiquetadas (véase «Validación del motor» en `research.md`), el motor clasifica correctamente limitación térmica confirmada, probable, por potencia, del equipo, mixta, sin limitación o indeterminada, asigna la gravedad y enumera las evidencias.

**Escenarios de aceptación:**

1. **Dado** el bit de razón `THERMAL` activo en al menos el 20 % de las muestras de una ventana estable con carga sostenida, **cuando** se diagnostica, **entonces** el diagnóstico es “Limitación térmica confirmada”.
2. **Dado** que no hay razones directas y la temperatura forma una meseta en el límite térmico efectivo, **cuando** se diagnostica, **entonces** el diagnóstico es “Evidencia compatible con limitación térmica”, nunca “confirmada”.
3. **Dado** margen térmico holgado y la potencia de paquete estable en una meseta (o los bits `PL1`/`PL2`/`EDP` activos), **cuando** se diagnostica, **entonces** la causa principal es potencia y no se recomienda mejorar la refrigeración como primera medida.
4. **Dado** temperatura elevada sin carga sostenida, o dentro de la ventana de turbo, **cuando** se diagnostica, **entonces** se advierte de la temperatura pero no se afirma pérdida de rendimiento.
5. **Dado** que concurren límites térmicos y eléctricos, **cuando** ambos tienen evidencia material, **entonces** el resultado se muestra como causa mixta y no fuerza una causa única (solo nivel A: sin límite de potencia medido no puede demostrarse una causa mixta).
6. **Dado** que la frecuencia cae al terminar la ventana de turbo (PL2 → PL1) con margen térmico, **cuando** se diagnostica, **entonces** se registra el evento informativo “fin del turbo de potencia (esperado)” y **no** se clasifica como limitación térmica.
7. **Dado** un portátil cuyo límite de potencia efectivo baja de forma progresiva durante la sesión con la temperatura de la CPU holgada, **cuando** se diagnostica, **entonces** el resultado es “Limitado por el equipo (gestión térmica del fabricante)” y la aplicación **sí** sugiere mejorar la ventilación del equipo.
8. **Dado** el bit `PROCHOT` activo sin el bit `THERMAL`, **cuando** se diagnostica, **entonces** el resultado es “Limitado por el equipo (señal externa)” y se sugiere revisar cargador, batería o alimentación, no la refrigeración.
9. **Dado** una limitación térmica con la frecuencia activa por encima de la frecuencia base, **cuando** se presenta, **entonces** se comunica como “rendimiento limitado por temperatura dentro de especificación” y no genera alerta; solo por debajo de la frecuencia base se comunica como problema.
10. **Dado** un juego que mantiene pocos núcleos al máximo con una carga total baja, **cuando** se diagnostica, **entonces** la evaluación se hace sobre esos núcleos activos y no se descarta por la carga total.

---

### Historia 3 — Estimar cuánto ayudaría mejorar la refrigeración (Prioridad P1)

Como usuario quiero saber, con honestidad, si mejorar la refrigeración me devolvería rendimiento y aproximadamente cuánto, y ver el rendimiento real que mi equipo sostiene cuando hago una prueba.

**Por qué es prioritaria:** responde a la pregunta principal del producto, pero la versión inicial (razón de relojes frente a un baseline) mezclaba el fin del turbo, el tipo de carga y la calidad del baseline con el efecto del calor. Esta historia sustituye esa cifra por dos medidas con fundamento físico o medido.

**Prueba independiente:** con una traza de nivel A limitada térmicamente, se calcula el potencial por techo de potencia con su método, tramo y confianza; sin límite de potencia conocido se omite la cifra; con una sesión guiada se muestra el rendimiento medido de la carga de prueba.

**Escenarios de aceptación:**

1. **Dado** una limitación térmica, mixta o del equipo (chasis) fuera de la ventana de turbo, con límite de potencia efectivo y potencia medidos, **cuando** la potencia real queda por debajo del límite, **entonces** se muestra el potencial con mejor refrigeración como tramo (`apenas`, `moderado`, `notable`) y un rango redondeado hacia fuera a múltiplos de 5 %.
2. **Dado** que no se conoce el límite de potencia efectivo, **cuando** hay limitación térmica, **entonces** se muestra la limitación, su duración y su gravedad, pero ninguna cifra; si la gravedad es «por debajo de la frecuencia base» se añade el tramo cualitativo «probablemente notable» con confianza baja.
3. **Dado** una limitación de potencia pura, **cuando** se presenta, **entonces** se indica que la refrigeración apenas influye y no se cuantifica.
4. **Dado** un diagnóstico guiado completado, **cuando** se muestra el resultado, **entonces** aparece el rendimiento sostenido **medido** por el generador de carga en relación con el inicial, desglosado por causa (fin del turbo, potencia, temperatura, equipo), rotulado como «rendimiento de la carga de prueba».
5. **Dado** dos diagnósticos guiados del mismo perfil y contexto energético, **cuando** el usuario compara «antes/después», **entonces** se comparan sus rendimientos medidos, no relojes.
6. **Dado** un procesador híbrido, **cuando** se estima o mide, **entonces** el cálculo se realiza por grupos y solo con los núcleos activos.
7. **Dado** cualquier cifra de potencial o rendimiento, **cuando** se presenta, **entonces** nunca lleva decimales ni un rango más estrecho de 5 puntos.

---

### Historia 4 — Ejecutar un diagnóstico guiado (Prioridad P2)

Como usuario quiero una sesión guiada de varios minutos que compare el comportamiento frío y caliente del mismo equipo.

**Por qué es prioritaria:** mejora la calidad de la referencia y hace repetible el diagnóstico.

**Prueba independiente:** el usuario inicia, observa y cancela una sesión; la sesión finaliza con informe o se detiene de forma segura.

**Escenarios de aceptación:**

1. **Dado** que el usuario acepta la advertencia, **cuando** inicia la prueba, **entonces** se registran calentamiento, estado estable y recuperación con progreso y cancelación visibles.
2. **Dado** que se alcanza un límite de parada, desaparece el sensor crítico o falla el auxiliar, **cuando** la prueba está activa, **entonces** la carga se detiene inmediatamente y el informe explica el motivo.
3. **Dado** que el usuario cancela, **cuando** pulsa detener, **entonces** cesa la carga y los datos parciales se conservan marcados como incompletos.
4. **Dado** un portátil en batería o modo ahorro, **cuando** se prepara la prueba, **entonces** se advierte que el resultado puede reflejar límites energéticos y el usuario puede cancelar.
5. **Dado** que el acceso avanzado deja de funcionar a mitad de una prueba guiada, **cuando** la prueba continúa, **entonces** termina con normalidad, el informe muestra el instante del cambio y el nivel final, y la cifra de rendimiento y la opción de marcar como referencia solo aparecen si el nivel A se mantuvo durante toda la prueba; en caso contrario el informe explica por qué no hay cifra.

---

### Historia 5 — Explorar la evolución y la causa (Prioridad P2)

Como usuario técnico quiero inspeccionar gráficos sincronizados para entender cuándo y por qué cayó el rendimiento.

**Por qué es prioritaria:** hace auditable el diagnóstico y satisface el requisito visual.

**Prueba independiente:** una sesión grabada se representa con cursor sincronizado, eventos y detalle por núcleo.

**Escenarios de aceptación:**

1. **Dado** un historial con eventos, **cuando** el usuario mueve el cursor temporal, **entonces** temperatura, carga, relojes y potencia muestran valores del mismo instante.
2. **Dado** un evento térmico o eléctrico, **cuando** aparece en la línea temporal, **entonces** existe una banda o marcador con tipo, duración y evidencia.
3. **Dado** que hay temperaturas/relojes por núcleo, **cuando** se abre la vista de topología, **entonces** se muestran como mapa térmico con etiqueta y valor accesibles.
4. **Dado** un tramo sin datos, **cuando** se dibuja, **entonces** se representa como hueco y no como cero ni como línea interpolada engañosa.

---

### Historia 6 — Supervisar en segundo plano y recibir alertas (Prioridad P2)

Como usuario quiero minimizar la aplicación a la bandeja y recibir avisos solo cuando exista una situación relevante y persistente.

**Por qué es prioritaria:** permite descubrir problemas durante cargas reales.

**Prueba independiente:** el servicio de monitorización continúa con la ventana cerrada, respeta frecuencia y silencio, y no duplica avisos.

**Escenarios de aceptación:**

1. **Dado** el modo bandeja activo, **cuando** se cierra la ventana, **entonces** el muestreo continúa y el icono refleja el estado general.
2. **Dado** un evento breve, **cuando** no supera la duración mínima, **entonces** no genera una notificación alarmista.
3. **Dado** un evento persistente, **cuando** supera las reglas de aviso, **entonces** se envía como máximo una notificación por episodio y existe periodo de enfriamiento.

---

### Historia 7 — Exportar evidencia y configurar privacidad (Prioridad P3)

Como usuario quiero exportar una sesión para compararla o compartirla sin revelar datos innecesarios.

**Por qué es prioritaria:** facilita soporte, comparativas antes/después y análisis externo.

**Prueba independiente:** una sesión puede exportarse e importarse con fidelidad; la anonimización elimina identificadores seleccionados.

**Escenarios de aceptación:**

1. **Dado** un informe terminado, **cuando** se exporta, **entonces** el usuario puede elegir CSV de muestras y JSON de diagnóstico.
2. **Dado** el modo anónimo, **cuando** se exporta, **entonces** se excluyen nombre del equipo, números de serie, usuario y rutas locales.
3. **Dado** un archivo de sesión compatible, **cuando** se importa, **entonces** puede reproducirse sin consultar sensores reales.

---

### Historia 8 — Comprender y preparar la aplicación en el primer inicio (Prioridad P1)

Como usuario no técnico quiero una introducción breve que explique qué observa ThrottleWatch, qué puede concluir y qué decisiones dependen de mí, para empezar a usarla sin conocimientos previos.

**Por qué es prioritaria:** la aplicación emplea conceptos técnicos y capacidades de hardware variables; una primera experiencia clara evita falsas expectativas y permisos inesperados.

**Prueba independiente:** en una instalación limpia se recorren, omiten y reanudan cinco diapositivas; la última detecta el equipo de forma pasiva y permite entrar en `Ahora` incluso con cobertura parcial.

**Escenarios de aceptación:**

1. **Dado** un perfil sin onboarding completado, **cuando** se abre la aplicación, **entonces** aparecen cinco diapositivas sobre propósito, señales, diagnóstico prudente, privacidad/preferencias y detección del equipo.
2. **Dado** un término como *thermal throttling*, límite térmico (TjMax), frecuencia base o referencia, **cuando** aparece en el recorrido, **entonces** incluye una explicación breve en lenguaje corriente.
3. **Dado** que el usuario abandona a mitad del recorrido, **cuando** vuelve a abrirlo, **entonces** continúa en la última diapositiva alcanzada.
4. **Dado** que el usuario pulsa omitir, **cuando** lo hace desde cualquiera de las cuatro primeras diapositivas, **entonces** entra en `Ahora` sin diálogo de confirmación y la detección pasiva continúa allí sin solicitar elevación.
5. **Dado** que faltan sensores avanzados, **cuando** termina la detección, **entonces** se explica la cobertura reducida y se ofrece instalar o reparar el acceso solo mediante una acción explícita.
6. **Dado** que el recorrido termina, **cuando** se presenta el siguiente paso, **entonces** `Abrir Ahora` es la acción principal y el diagnóstico guiado es una acción secundaria que nunca comienza automáticamente.
7. **Dado** un cambio importante posterior de permisos o comportamiento, **cuando** la aplicación actualizada se abre por primera vez, **entonces** muestra únicamente las novedades pertinentes y no repite todo el recorrido.

---

### Historia 9 — Adaptar idioma, apariencia y ventana (Prioridad P1)

Como usuario quiero que la aplicación use de entrada un idioma y tema apropiados, y recuerde cómo dejé su ventana, para que se integre de manera natural en Windows.

**Por qué es prioritaria:** idioma, legibilidad y comportamiento de ventana afectan a toda la experiencia, incluido el primer inicio y los errores.

**Prueba independiente:** se simulan locales y temas de Windows, se cambia cada preferencia y se reinicia la aplicación verificando idioma, tema y geometría restaurados.

**Escenarios de aceptación:**

1. **Dado** el modo de idioma `Sistema`, **cuando** Windows usa español, catalán/valenciano, gallego, euskera, asturiano o aragonés, **entonces** toda la interfaz se muestra en español, incluido catalán de Andorra.
2. **Dado** cualquier otro idioma de Windows, incluido portugués, **cuando** se inicia la aplicación, **entonces** toda la interfaz se muestra en inglés.
3. **Dado** que el usuario elige manualmente `Español` o `English`, **cuando** reinicia, **entonces** esa elección prevalece sobre Windows; en modo `Sistema`, un cambio de idioma del sistema se aplica en el siguiente inicio.
4. **Dado** el tema `Sistema`, **cuando** Windows cambia entre claro y oscuro, **entonces** la aplicación adopta el tema activo; los valores manuales `Claro` y `Oscuro` prevalecen sobre el sistema.
5. **Dado** que el usuario mueve, redimensiona o maximiza la ventana, **cuando** vuelve a abrir la aplicación, **entonces** se restauran posición, tamaño, estado maximizado y último tamaño restaurado.
6. **Dado** que cambió la disposición de monitores, **cuando** la posición guardada ya no es visible, **entonces** la ventana se recoloca dentro de la pantalla principal.
7. **Dado** cualquier estado de la aplicación, incluido onboarding o error inicial, **cuando** se usa la barra superior, **entonces** permite arrastrar, minimizar, maximizar/restaurar, cerrar y maximizar/restaurar con doble clic, con controles accesibles.
8. **Dado** Windows al 200 % de escala en un monitor cuya altura útil es inferior a 600 px lógicos, **cuando** se abre o se redimensiona la ventana, **entonces** esta cabe en el área útil sin bajar de 480×500 px lógicos y todas las acciones críticas siguen accesibles.
9. **Dado** un monitor cuya altura útil es inferior a 500 px lógicos (por ejemplo, 1366×768 al 200 %), **cuando** se abre la ventana, **entonces** se maximiza al área útil, el contenido se desplaza verticalmente y la barra de título, el banner global y `Detener ahora` permanecen fijos y visibles.

---

### Historia 10 — Configurar el comportamiento sin perder el control (Prioridad P2)

Como usuario quiero configurar monitorización, bandeja, avisos, privacidad y datos con opciones comprensibles, para adaptar ThrottleWatch sin tener que conocer frecuencias o detalles internos.

**Por qué es prioritaria:** varias capacidades consumen recursos, conservan datos o continúan tras cerrar la ventana y deben depender de decisiones visibles.

**Prueba independiente:** se cambian ajustes, se reinicia y se comprueba su persistencia, sus dependencias y el efecto de borrar datos o restaurar valores de fábrica.

**Escenarios de aceptación:**

1. **Dado** que no existe una acción de cierre guardada, **cuando** se pulsa la X por primera vez, **entonces** se pregunta entre salir y continuar en la bandeja midiendo, se guarda la elección y se indica dónde cambiarla; elegir bandeja activa la monitorización en segundo plano; cerrar el diálogo con `Esc` no guarda nada ni cierra la ventana.
2. **Dado** que la monitorización en bandeja está habilitada y la acción de cierre es `bandeja`, **cuando** el usuario deshabilita la monitorización en bandeja, **entonces** la acción de cierre pasa a `salir` en la misma operación y la interfaz lo comunica.
3. **Dado** `Iniciar con Windows` desactivado por defecto, **cuando** el usuario lo activa, **entonces** puede elegir iniciar mostrando la ventana o, si la monitorización de bandeja está habilitada, iniciar oculto.
4. **Dado** un usuario no técnico, **cuando** configura el muestreo, **entonces** elige entre `Bajo consumo`, `Normal` y `Diagnóstico`, dejando los valores numéricos en ajustes avanzados.
5. **Dado** una instalación nueva, **cuando** se consultan notificaciones y retención, **entonces** los avisos están desactivados y la retención es de siete días; también existen `solo sesión`, `1 día` y `30 días`.
6. **Dado** que el usuario elimina todos sus datos, **cuando** confirma, **entonces** se borran sesiones, muestras, informes, referencias y registros técnicos, pero se conservan preferencias y estado del onboarding.
7. **Dado** que el usuario restablece ThrottleWatch, **cuando** confirma una advertencia diferenciada, **entonces** se eliminan datos, registros técnicos y preferencias y el siguiente inicio se comporta como una instalación nueva.
8. **Dado** que el usuario activa `Registro detallado` en Ajustes › Acerca de y ayuda, **cuando** pasan 24 horas o reinicia la aplicación (lo que ocurra antes), **entonces** el registro vuelve solo al nivel normal; mientras está activo, su estado es visible.

---

### Historia 11 — Actualizar de forma voluntaria y verificable (Prioridad P3)

Como usuario quiero saber si existe una versión nueva y decidir por separado si la descargo y la instalo, para mantener ThrottleWatch actualizada sin tráfico ni cambios inesperados.

**Por qué es prioritaria:** facilita recibir correcciones, pero la aplicación debe seguir funcionando completamente sin red y preservar el control del usuario.

**Prueba independiente:** con el actualizador apagado no se produce tráfico; al activarlo se comprueba la versión, se descarga con progreso y solo se instala tras otra confirmación y una firma válida.

**Escenarios de aceptación:**

1. **Dado** el actualizador desactivado por defecto, **cuando** se usa la aplicación, **entonces** no realiza ninguna petición de actualización.
2. **Dado** el actualizador activo, **cuando** corresponde la comprobación automática, **entonces** consulta como máximo una vez cada 24 horas; `Buscar actualizaciones` permite un reintento manual sin esperar ese plazo.
3. **Dado** una versión nueva, **cuando** se detecta, **entonces** se avisa en Ajustes y mediante una notificación nativa, pero no se descarga nada.
4. **Dado** que el usuario pulsa descargar, **cuando** se reciben los bytes, **entonces** se muestra progreso y se verifica la firma Ed25519 antes de ofrecer instalar.
5. **Dado** un instalador válido, **cuando** el usuario pulsa instalar, **entonces** se solicita una confirmación independiente y se avisa de que ThrottleWatch se cerrará.
6. **Dado** un diagnóstico guiado o una exportación en curso, **cuando** se intenta instalar, **entonces** la instalación queda bloqueada hasta que la operación termine o se cancele.
7. **Dado** un fallo de red, descarga, firma o instalación, **cuando** se informa, **entonces** el resto de la aplicación continúa disponible y no se ofrece instalar un artefacto incompleto o no verificado.

## Casos límite

- CPU o placa no reconocida; sensores con nombres inesperados o duplicados.
- Máquina virtual o entorno sin acceso a sensores: la cobertura lo indica explícitamente como estado propio.
- Segunda instancia de la aplicación (se enfoca la existente y la nueva termina) o una actualización durante el muestreo.
- Temperatura disponible solo como paquete, solo por núcleo, `Tctl` con offset o margen respecto a TjMax.
- TjMax desconocido o dinámico; sensores que devuelven valores imposibles, congelados o intermitentes.
- Cambio de corriente alterna a batería, plan energético o modo OEM durante una sesión.
- Suspensión, hibernación, reanudación, cambio de hora o salto del reloj del sistema.
- Topología híbrida, SMT, núcleos aparcados o cambios de afinidad.
- Carga parcial, variable o concentrada en pocos núcleos.
- Sidecar detenido o protocolo incompatible (`collector_lost`); permisos insuficientes, controlador ausente o proveedor de bajo nivel que falla en mitad de una sesión (degradación a B/C según FR-090).
- Historial grande, disco lleno, base de datos dañada o exportación cancelada.
- Onboarding interrumpido, omitido o invalidado por una actualización importante.
- Locale de Windows ausente, no reconocido o cambiado mientras la aplicación está abierta.
- Tema del sistema cambiado en caliente; movimiento reducido y escalado elevados.
- Geometría guardada en un monitor desconectado o fuera del área visible.
- Cierre solicitado durante una prueba, exportación, descarga o instalación.
- Manifiesto de actualización inaccesible, versión no válida, firma incorrecta o descarga parcial.

## Requisitos

### Requisitos funcionales

- **FR-001**: El sistema DEBE descubrir procesador, topología y sensores disponibles al iniciar y tras reanudar el equipo.
- **FR-002**: El sistema DEBE registrar la procedencia, unidad, calidad y antigüedad de cada lectura.
- **FR-003**: El sistema DEBE obtener, cuando estén disponibles, carga por procesador lógico, temperatura, frecuencia activa, frecuencia base, potencia de paquete, límite térmico efectivo (TjMax y TCC offset), límites de potencia efectivos (PL1, PL2, Tau) y razones de limitación (`THERMAL`, `PROCHOT`, potencia, corriente).
- **FR-004**: El sistema DEBE normalizar sensores Intel y AMD a un modelo común sin perder el nombre original.
- **FR-005**: El sistema DEBE muestrear a 1 Hz por defecto y permitir perfiles de bajo consumo y diagnóstico.
- **FR-006**: El sistema DEBE conservar un búfer reciente en memoria y un historial local sujeto a retención configurable.
- **FR-007**: El sistema DEBE representar valores ausentes como desconocidos, nunca como cero.
- **FR-008**: El sistema DEBE calcular estados térmicos mediante margen al límite cuando exista y mediante reglas conservadoras cuando no exista.
- **FR-009**: El motor DEBE clasificar: normal, temperatura alta sin limitación demostrada, térmica probable, térmica confirmada, potencia/corriente, limitada por el equipo (gestión térmica del fabricante o señal externa), mixta e indeterminada; y DEBE asignar a toda limitación una gravedad `boost` (frecuencia activa ≥ base) o `below_base` (< base).
- **FR-010**: Todo diagnóstico DEBE incluir evidencias favorables, factores alternativos, confianza y el intervalo analizado (inicio, fin y duración con carga sostenida; en vivo, la ventana estable en curso).
- **FR-011**: El sistema NO DEBE interpretar el porcentaje de tiempo con throttling como porcentaje de rendimiento perdido.
- **FR-012**: El sistema NO DEBE usar el turbo máximo anunciado como referencia de rendimiento multinúcleo sostenido.
- **FR-013**: El potencial con mejor refrigeración DEBE calcularse con el método de techo de potencia (límite de potencia efectivo y potencia de paquete medidos) y solo fuera de la ventana de turbo; el rendimiento en porcentaje solo DEBE mostrarse cuando lo mida directamente el generador de carga del diagnóstico guiado. No existe estimación basada en baselines aprendidos.
- **FR-014**: Si faltan las entradas de un método, el sistema DEBE omitir la cifra y explicar qué falta.
- **FR-015**: Toda cifra DEBE presentarse como tramo y como rango redondeado hacia fuera a múltiplos de 5 %, con método y confianza, sin decimales.
- **FR-016**: El sistema DEBE separar grupos de núcleos heterogéneos y ponderar solo grupos activos.
- **FR-017**: La aplicación DEBE ofrecer una prueba guiada opcional, cancelable y con límites de parada.
- **FR-018**: La prueba NO DEBE modificar configuración de firmware, voltajes, potencia ni ventiladores.
- **FR-019**: El panel principal DEBE mostrar estado, temperatura, margen térmico, carga, frecuencia activa, potencia disponible, gravedad y resumen de diagnóstico.
- **FR-020**: La vista de análisis DEBE incluir series temporales sincronizadas y eventos superpuestos.
- **FR-021**: La vista de CPU DEBE incluir mapa de núcleos cuando existan datos por núcleo.
- **FR-022**: La interfaz DEBE ofrecer detalle progresivo desde lenguaje natural hasta sensores originales.
- **FR-023**: El sistema DEBE mostrar una matriz de cobertura del equipo y acciones para recuperar sensores, si procede.
- **FR-024**: El modo bandeja DEBE continuar el muestreo solo si el usuario lo habilita.
- **FR-025**: Las notificaciones DEBEN exigir persistencia, deduplicarse y respetar el periodo de silencio.
- **FR-026**: El sistema DEBE exportar muestras CSV y un informe JSON versionado.
- **FR-027**: El sistema DEBE importar y reproducir sesiones compatibles sin hardware real.
- **FR-028**: La exportación DEBE ofrecer anonimización previa y describir qué campos se omiten.
- **FR-029**: El sistema DEBE funcionar completamente sin conexión y sin telemetría remota por defecto.
- **FR-030**: La interfaz principal DEBE ejecutarse sin privilegios administrativos.
- **FR-031**: El sistema DEBE registrar fallos del colector y del diagnóstico sin incluir secretos, identificadores personales ni del equipo, según el principio XVII de la constitución.
- **FR-032**: Preferencias, referencias guiadas, tabla de límites térmicos y reglas aplicadas DEBEN conservar su versión para interpretar diagnósticos históricos.
- **FR-033**: Toda la interfaz, incluidos onboarding, bandeja, notificaciones, errores y diálogos nativos propios, DEBE estar disponible en español e inglés desde el MVP, sin literales visibles fuera de los catálogos de traducción.
- **FR-034**: El primer inicio DEBE ofrecer un recorrido de cinco diapositivas, omitible, reanudable y repetible desde Ajustes.
- **FR-035**: El onboarding DEBE explicar propósito, señales, límites del diagnóstico, privacidad, preferencias esenciales y cobertura del equipo para un usuario no técnico, definiendo brevemente los términos técnicos que utilice.
- **FR-036**: La detección pasiva DEBE comenzar automáticamente en la quinta diapositiva; omitir antes DEBE trasladar la detección normal a `Ahora` sin elevación automática.
- **FR-037**: La aplicación DEBE guardar versión, finalización y última diapositiva del onboarding; una actualización importante PUEDE mostrar solo novedades versionadas pertinentes.
- **FR-038**: El modo de idioma inicial DEBE ser `Sistema`, con opciones manuales `Español` y `English`; la selección manual DEBE prevalecer sobre Windows.
- **FR-039**: En modo `Sistema`, los locales `es-*`, `ca-*`, `gl-*`, `eu-*`, `ast-*` y `an-*` DEBEN resolverse a español, incluido `ca-AD`; cualquier otro locale, incluido `pt-*`, DEBE resolverse a inglés.
- **FR-040**: Los cambios de locale de Windows en modo `Sistema` DEBEN aplicarse en el siguiente inicio; los cambios manuales de idioma DEBEN aplicarse sin perder el contexto de navegación.
- **FR-041**: CSV y JSON DEBEN usar claves y enums técnicos estables en inglés; todo informe destinado a personas DEBE usar el idioma efectivo elegido.
- **FR-042**: La apariencia DEBE ofrecer `Sistema`, `Claro` y `Oscuro`, usando `Sistema` por defecto y reaccionando a cambios de tema de Windows mientras la aplicación está abierta.
- **FR-043**: El movimiento reducido DEBE seguir Windows por defecto y poder anularse desde Ajustes (`Sistema` / `Reducido` / `Completo`).
- **FR-043b**: El efecto de vidrio de la interfaz DEBE ofrecer `Sistema`, `Completo`, `Reducido` y `Sin`; `Sistema` DEBE seguir el ajuste «Efectos de transparencia» de Windows. La aplicación DEBE degradar el nivel automáticamente según los umbrales `glass.*` de los parámetros iniciales y DEBE mantener el contraste AA del texto en todos los niveles.
- **FR-044**: La ventana principal DEBE usar una barra de título propia con icono, nombre, región de arrastre, minimizar, maximizar/restaurar, cerrar y doble clic para maximizar/restaurar, disponible también en onboarding y errores iniciales.
- **FR-045**: La aplicación DEBE conservar posición, tamaño, estado maximizado y último tamaño restaurado; nunca DEBE restaurar la ventana completamente fuera del área visible ni recordar un estado minimizado.
- **FR-046**: En el primer cierre sin decisión guardada, el sistema DEBE preguntar si sale o continúa en la bandeja, guardar obligatoriamente la elección e indicar que puede cambiarse en Ajustes. Elegir bandeja DEBE activar `tray.monitoring_enabled`; no existe una **configuración** «en bandeja sin muestreo» (`lifecycle.close_action = tray` implica `tray.monitoring_enabled = true`); la pausa manual desde el menú de bandeja (FR-059) es un estado transitorio que no se persiste y se anula al mostrar la ventana o reiniciar. Cerrar el diálogo sin elegir DEBE dejar la decisión sin guardar y la ventana abierta.
- **FR-047**: `Iniciar con Windows` DEBE estar desactivado por defecto y, al activarse, permitir mostrar la ventana o iniciar oculto; iniciar oculto solo DEBE permitirse si la monitorización en bandeja está habilitada.
- **FR-048**: Los avisos DEBEN estar desactivados por defecto; el onboarding solo DEBE informar de que pueden habilitarse en Ajustes.
- **FR-049**: La configuración de muestreo DEBE presentar los perfiles `Bajo consumo`, `Normal` y `Diagnóstico`; sus frecuencias concretas solo aparecen en ajustes avanzados.
- **FR-050**: `Eliminar todos mis datos` DEBE borrar sesiones, muestras, eventos, informes, referencias guiadas y registros técnicos tras confirmación, conservando preferencias y estado del onboarding.
- **FR-051**: `Restablecer ThrottleWatch` DEBE requerir una confirmación diferenciada, borrar datos, registros técnicos y preferencias y provocar un primer inicio limpio.
- **FR-052**: La comprobación de actualizaciones DEBE estar desactivada por defecto y, mientras lo esté, NO DEBE realizar tráfico de red.
- **FR-053**: Con actualizaciones activas, el sistema DEBE comprobar automáticamente como máximo una vez cada 24 horas y ofrecer una comprobación manual que pueda reintentar antes de ese plazo.
- **FR-054**: Detectar una versión, descargarla e instalarla DEBEN ser tres gestos separados; ningún paso DEBE iniciar automáticamente el siguiente.
- **FR-055**: El instalador descargado DEBE validarse con firma Ed25519 antes de ofrecer instalación; un artefacto parcial o inválido DEBE descartarse.
- **FR-056**: Una actualización disponible DEBE aparecer en Ajustes y como notificación nativa; la descarga DEBE mostrar progreso.
- **FR-057**: Instalar DEBE exigir confirmación, avisar del cierre y quedar bloqueado mientras haya un diagnóstico guiado, exportación, importación o borrado/restablecimiento en curso, mostrando el motivo del bloqueo.
- **FR-058**: El actualizador DEBE usar un endpoint fijo de GitHub Releases, ejecutarse desde el backend, no enviar identificadores ni telemetría y aislar todos sus fallos del resto de la aplicación. No existe selección de canal: solo se publican versiones estables.
- **FR-059**: El icono de bandeja DEBE distinguir cinco estados: `normal`, `aviso`, `crítico`, `desconocido` y `desconectado`, mapeados desde la clasificación en vivo y el estado del colector. Un clic DEBE mostrar u ocultar la ventana; el menú contextual DEBE ofrecer estado actual, abrir, pausar/reanudar muestreo y salir. Con el muestreo pausado el icono usa el estado `desconocido` con el texto «Muestreo en pausa» en el tooltip y en el menú; no se emiten alertas mientras dure la pausa.
- **FR-060**: El comportamiento en batería DEBE ser configurable mediante `sampling.on_battery` con valores `keep` (mantener), `low_power` (bajo consumo) y `pause` (pausar); el valor inicial es `keep`. Un cambio de fuente de alimentación DEBE aplicarse en la siguiente muestra.
- **FR-061**: Deshabilitar la monitorización en bandeja DEBE corregir atómicamente cualquier configuración dependiente incompatible (`startup.mode=tray` → `window`, `lifecycle.close_action=tray` → `exit`) y comunicarlo en la respuesta.
- **FR-062**: Un control dependiente de otro ajuste DEBE permanecer visible, deshabilitado y con una explicación de la condición que lo habilita; nunca DEBE ocultarse.
- **FR-063**: `Eliminar todos mis datos` y `Restablecer ThrottleWatch` DEBEN ejecutarse como transacciones; un fallo parcial DEBE revertir lo posible y comunicar qué no pudo completarse.
- **FR-064**: El mapa de núcleos DEBE permitir alternar la magnitud representada entre temperatura y reloj sin perder la selección de núcleo.
- **FR-065**: Tras una reanudación desde suspensión o hibernación, el sistema DEBE marcar el hueco, repetir el descubrimiento de capacidades y abrir una sesión pasiva nueva, conservando la interfaz utilizable en todo momento.
- **FR-066**: La vista de análisis DEBE ofrecer navegación del cursor y selección de rango por teclado, un resumen textual del rango y una tabla accesible equivalente a las series dibujadas.
- **FR-067**: Una sesión pasiva DEBE comenzar al iniciar el muestreo, tras un hueco superior a 60 s y, como máximo, cada 24 h de duración continua. Su informe DEBE congelarse al cerrarla; mientras está activa, el diagnóstico visible es «en vivo» y se marca como provisional. La retención `solo sesión` DEBE borrar los datos y los registros técnicos al salir de la aplicación.
- **FR-068**: La métrica de frecuencia del motor DEBE ser la **frecuencia activa** por procesador lógico (frecuencia mientras ejecuta, sin incluir el reposo), obtenida de `% Processor Performance` × `Processor Frequency` de Windows (calidad `derived`) o de una lectura directa equivalente; el reloj nominal del colector solo DEBE usarse como `substitute`. La interfaz DEBE llamarla «frecuencia activa» y no presentarla como el «effective clock» de otras herramientas.
- **FR-069**: `térmica confirmada` DEBE exigir el bit de razón `THERMAL` (o su equivalente directo del fabricante). `PROCHOT` sin `THERMAL`, la meseta de temperatura o el margen agotado NO DEBEN producir `confirmada`. En AMD, el equivalente directo es la tabla PM del SMU (solo versiones de la lista permitida): `thermal_flag` se activa cuando el valor THM es ≥ 99 % de su límite **y** PPT, TDC y EDC están por debajo del 95 % de los suyos (el limitador térmico del firmware es el que manda). Hasta que el corpus AMD valide esta equivalencia (T045), la confianza de `thermal_confirmed` en AMD tiene techo `media`.
- **FR-070**: La confianza máxima DEBE depender del nivel de cobertura del equipo: nivel A → alta, nivel B → media, nivel C → baja; un reloj `substitute` limita a baja.
- **FR-071**: El contexto energético (fuente de alimentación, plan energético activo y porcentaje de batería cuando exista) DEBE registrarse con cada muestra y estar disponible para el motor y para los informes.
- **FR-072**: El usuario DEBE poder marcar un diagnóstico guiado completado como referencia para comparaciones «antes/después» desde Sesiones o desde el Informe, tras confirmación, y DEBE poder retirar esa marca. Las sesiones pasivas no pueden ser referencia.
- **FR-073**: Una sesión importada DEBE conservar su informe original y ofrecer una reevaluación con las reglas actuales que se muestra junto al original; su resultado nunca DEBE usarse como referencia de la CPU local.
- **FR-074**: Los números y fechas DEBEN formatearse según el idioma efectivo de la interfaz; la temperatura se expresa exclusivamente en grados Celsius en el MVP.
- **FR-076**: El límite térmico efectivo DEBE ser TjMax menos el offset de activación térmica (TCC offset) cuando el equipo lo exponga; en AMD, el límite de la familia según una tabla versionada o el que informe el colector. El margen siempre se mide contra ese límite efectivo.
- **FR-077**: Tras cada inicio de carga, el motor DEBE excluir de la decisión de limitación sostenida la ventana de turbo (`max(Tau, 60 s)`) y registrar un escalón de potencia a la baja con carga constante como evento informativo `turbo_end`, nunca como limitación.
- **FR-078**: Sin razones directas, el motor DEBE atribuir la causa por la magnitud que se estabiliza: meseta de temperatura en el límite → térmica; meseta de potencia con margen → potencia; la causa mixta exige razones directas (nivel A). No DEBE usar la caída de frecuencia por sí sola como evidencia térmica.
- **FR-079**: `limitada por el equipo` DEBE distinguir el subtipo `chassis_thermal` (límite de potencia efectivo que baja durante la sesión con la CPU con margen) y `external_prochot` (`PROCHOT` sin `THERMAL`); para el primero DEBE sugerir mejorar la ventilación del equipo y para el segundo revisar alimentación, cargador o batería.
- **FR-080**: Solo las limitaciones de gravedad `below_base` DEBEN generar alertas; las de gravedad `boost` se comunican como comportamiento dentro de especificación.
- **FR-081**: La condición de carga sostenida y la frecuencia activa DEBEN evaluarse sobre los núcleos activos (utilidad ≥ 80 %), no sobre la carga total.
- **FR-082**: La cobertura DEBE mostrar el nivel del equipo (A completo, B con potencia, C básico) y lo que cada nivel permite concluir; el onboarding y la cobertura DEBEN presentar el acceso avanzado como recomendado para alcanzar el nivel A, siempre mediante una acción explícita.
- **FR-083**: El generador de carga del diagnóstico guiado DEBE medir el trabajo completado por hilo y por segundo, y el informe guiado DEBE mostrar el rendimiento sostenido relativo al inicial desglosado por causa. Si el ADR de T054 aprueba solo la **observación externa**, no existe cifra de rendimiento: el informe guiado muestra clasificación, gravedad y relojes rotulados como observación, sin porcentaje; US3-4, US3-5, la comparación antes/después y la parte guiada de SC-002 quedan diferidas y así se registra en el ADR.
- **FR-084**: Las razones de limitación DEBEN leerse mediante sus bits de registro (log), que se limpian tras cada lectura, para medir si ocurrieron desde la muestra anterior y no solo en el instante de leer.
- **FR-085**: La prueba guiada NO DEBE detenerse por alcanzar el límite térmico, que es el fenómeno que mide; DEBE detenerse si la temperatura supera el límite efectivo en más de 2 °C durante 3 muestras (el control térmico del procesador no actúa), si permanece en el límite con frecuencia activa inferior al 50 % de la base durante 10 s (refrigeración gravemente insuficiente), si se pierde el sensor crítico o si el generador deja de responder.
- **FR-086**: Ajustes › Acerca de y ayuda DEBE ofrecer `Registro detallado`, que eleva el registro técnico al nivel `debug` y se desactiva automáticamente a las 24 horas o al reiniciar la aplicación, lo que ocurra antes. Mientras esté activo, su estado DEBE ser visible. El nivel de registro en la versión instalada NO DEBE poder cambiarse por variables de entorno ni argumentos (constitución XV y XVII).
- **FR-087**: La acción explícita «instalar acceso avanzado» DEBE ejecutar el instalador oficial de PawnIO empaquetado con ThrottleWatch (versión fijada y firma verificada antes de lanzarlo), con una única petición UAC por máquina; ThrottleWatch NO DEBE modificar ni sustituir el controlador ni descargarlo en tiempo de ejecución. La licencia de redistribución DEBE constar en `THIRD-PARTY-NOTICES`.
- **FR-088**: Si el proveedor de acceso de bajo nivel solo admite procesos elevados, el nivel A DEBE obtenerse mediante un lanzador elevado exclusivo del sidecar, registrado en el mismo paso por máquina de FR-087 (sin UAC adicional); la interfaz DEBE seguir sin privilegios (FR-030) y NO DEBE solicitar UAC en el arranque diario. El mecanismo concreto (tarea programada o servicio mínimo) y su revisión de amenazas DEBEN constar en un ADR antes de implementarse; el usuario DEBE poder desactivar el acceso avanzado desde Ajustes, con vuelta a nivel B/C.
- **FR-089**: Si ya existe un PawnIO en la máquina con versión igual o superior a la mínima probada por ThrottleWatch (versión fijada junto al instalador empaquetado), el sistema DEBE reutilizarlo sin instalar ni pedir UAC; si la versión es anterior, la cobertura DEBE ofrecer actualizarlo solo mediante acción explícita. Desinstalar ThrottleWatch NUNCA DEBE eliminar PawnIO; el desinstalador DEBE indicar que es un controlador compartido y cómo eliminarlo por separado.
- **FR-090**: Si el acceso avanzado deja de estar operativo durante una sesión (servicio detenido, lectura denegada o error del proveedor), el sidecar DEBE continuar muestreando en nivel B/C sin interrumpir la sesión; la sesión DEBE registrar cada cambio de nivel de cobertura con su marca de tiempo y el motor DEBE evaluar cada ventana con el nivel vigente en ella (las cifras que exigen nivel A se detienen desde ese instante). La cobertura DEBE mostrar el estado con un aviso no intrusivo y la opción de reparar por acción explícita; la alerta `collector_lost` se reserva a la pérdida completa del sidecar. En una prueba guiada, la prueba continúa hasta el final; el informe guiado DEBE registrar el instante del cambio y el nivel final, y la cifra de rendimiento y la marca de referencia solo DEBEN ofrecerse si el nivel A se mantuvo durante toda la prueba.
- **FR-075**: Cuando el almacenamiento no esté disponible (disco lleno o base de datos dañada), el muestreo DEBE continuar en memoria, la interfaz DEBE avisar de forma persistente y el sistema DEBE reintentar o recuperar el almacenamiento sin intervención destructiva automática sobre los datos existentes.

### Requisitos no funcionales

- **NFR-001 — Fluidez:** animaciones y navegación a 60 fps en hardware objetivo; los gráficos pueden actualizarse a la frecuencia de muestreo sin bloquear la UI. El material de vidrio (desenfoque de fondo) NO DEBE elevar el consumo pasivo por encima de NFR-003; si lo hace, el nivel baja a `reducido`.
- **NFR-002 — Latencia:** una muestra válida debe verse en menos de 1,5 s en el percentil 95.
- **NFR-003 — Consumo:** monitorización pasiva con objetivo inferior al 1 % de CPU promedio y 180 MB de memoria en un equipo de referencia moderno.
- **NFR-004 — Resiliencia:** la caída del colector no debe cerrar la interfaz; debe reiniciarse con retroceso limitado según los parámetros `collector.*` o pedir intervención.
- **NFR-005 — Accesibilidad:** WCAG 2.2 nivel AA según el principio XII de la constitución: navegación completa por teclado, foco visible y no oculto, contraste (texto ≥ 4,5:1; texto grande, iconos y trazos ≥ 3:1), estados no dependientes solo del color, lectores de pantalla (Narrador y NVDA), temas de contraste de Windows, objetivos ≥ 24×24 px y movimiento reducido.
- **NFR-006 — Seguridad:** IPC local con autenticación efímera, esquema validado y lista cerrada de comandos.
- **NFR-007 — Retención:** siete días por defecto; el usuario puede elegir 1, 7, 30 días o solo sesión.
- **NFR-008 — Compatibilidad:** Windows 11 24H2 y 25H2 x64 y Windows 10 22H2 x64 mientras WebView2 y .NET 10 lo admitan oficialmente, con WebView2 Runtime Evergreen (constitución, «Plataforma objetivo»). Solo x64 en el MVP; otras arquitecturas requieren enmienda de la constitución.
- **NFR-009 — Auditabilidad:** el mismo conjunto de muestras y reglas debe producir el mismo diagnóstico.
- **NFR-010 — Inicio:** primera información útil en menos de 5 s en un equipo compatible típico.
- **NFR-011 — Localización:** el 100 % de las claves visibles DEBE existir en ambos catálogos y las variantes inglesa y española no DEBEN romper la ventana mínima ni el escalado de Windows al 200 %, incluido el mínimo reducido a 480×500 px lógicos cuando la altura útil del monitor es inferior a 600.
- **NFR-012 — Ventana:** los controles propios DEBEN ser accesibles por teclado y lector de pantalla y conservar las convenciones esperadas de minimizar, maximizar/restaurar, doble clic y cierre.
- **NFR-013 — Actualizaciones:** toda versión instalable DEBE estar firmada y las operaciones de red NO DEBEN bloquear la UI, el muestreo ni el cierre seguro del colector.
- **NFR-014 — Coherencia de interfaz:** todas las pantallas DEBEN reutilizar el vocabulario visual y los patrones aprobados, sin variantes locales casi equivalentes, y conservar comportamiento y jerarquía en español e inglés, temas claro y oscuro y tamaños compacto, medio y expandido. Los estados aplicables de carga, vacío, datos degradados y error DEBEN estar definidos y ser accesibles.
- **NFR-015 — Apariencia nativa:** ningún botón, fila con estilo de enlace, pestaña o chip DEBE mostrar subrayado al pasar el puntero o recibir foco, ni DEBE usar el cursor de mano; ambos se reservan exclusivamente para la superficie de trazado de `Análisis`. La navegación y las acciones dentro de la aplicación DEBEN implementarse con controles de botón, nunca con hiperenlaces de navegador, salvo para abrir contenido fuera de la aplicación.
- **NFR-016 — Almacenamiento:** con perfil `Normal` y sin historial por núcleo, ≤ 40 MB por día de monitorización continua; con la retención inicial de 7 días, la base de datos NO DEBE superar 300 MB. Una sesión guiada (con detalle por núcleo) ≤ 15 MB. Son objetivos iniciales que el spike de sensores (T019) debe validar; superar el presupuesto en la medición de T108 bloquea la entrega.

### Parámetros iniciales (ruleset v1)

Los valores siguientes son los iniciales de la versión 1 de reglas. Se almacenan en configuración versionada (`ruleset-v1`), son revisables tras validar el corpus de trazas y cualquier cambio exige nueva versión de reglas. Su presencia aquí evita que la implementación los invente.

Cada parámetro tiene un identificador estable en inglés (`grupo.nombre`) que coincide con la clave de `ruleset-v1.json` y con la entrada de `traceability.md`. La tabla siguiente los enumera; ningún número de decisión puede existir en el código sin uno (constitución VII).

| Grupo | Identificadores (valor inicial) |
|---|---|
| Estado térmico | `thermal.margin_warn_c` (8), `thermal.margin_critical_c` (3), `thermal.abs_warn_c` (85), `thermal.abs_critical_c` (95) |
| Núcleos activos y carga | `load.active_core_util_pct` (80), `load.start_threshold_pct` (30) |
| Ventanas | `window.stable_s` (60), `window.step_s` (10), `window.turbo_min_s` (60) |
| Ventana de turbo | `turbo.end_power_drop_pct` (15), `turbo.end_drop_window_s` (5) |
| Mesetas | `plateau.thermal_margin_c` (3), `plateau.thermal_stddev_c` (1,5), `plateau.power_cv_pct` (3) |
| Reglas | `rules.reason_occupancy_pct` (20), `rules.chassis_limit_drop_pct` (10), `rules.chassis_plateau_drop_pct` (15), `rules.chassis_min_steps` (2), `rules.chassis_min_span_s` (180), `rules.oem_step_window_s` (5), `rules.power_margin_a_c` (3), `rules.power_margin_b_c` (8), `rules.freq_drop_vs_turbo_pct` (8) |
| Gravedad | `severity.below_base_ratio` (0,97), `severity.min_duration_s` (30) |
| Sesión | `session.indeterminate_share_pct` (50), `session.class_min_s` (60), `session.class_min_share_pct` (10), `session.gap_s` (60), `session.max_h` (24) |
| Confianza | `confidence.medium_min` (0,45), `confidence.high_min` (0,75) |
| Potencial | `potential.range_low_factor` (0,5), `potential.tier_barely_max_pct` (3), `potential.tier_moderate_max_pct` (10), `potential.round_step_pct` (5) |
| Prueba guiada | `guided.preflight_max_s` (30), `guided.rest_s` (60), `guided.warmup_s` (90), `guided.load_s` (short 180, standard 240, long 360), `guided.recovery_s` (120), `guided.measure_head_s` (20), `guided.measure_tail_s` (120), `guided.stop_over_limit_c` (2), `guided.stop_over_limit_samples` (3), `guided.stop_low_freq_ratio` (0,5), `guided.stop_low_freq_s` (10), `guided.stop_missing_sensor_samples` (3), `guided.stop_generator_timeout_s` (5) |
| Alertas | `alerts.min_persistence_s` (90), `alerts.cooldown_min` (30) |
| Muestreo | `sampling.interval_ms` (low_power 5000, normal 1000, diagnostic 500) |
| Vidrio | `glass.degrade_fps` (50), `glass.degrade_window_s` (3), `glass.degrade_idle_cpu_pct` (1), `glass.degrade_idle_window_s` (30), `glass.restore_fps` (55), `glass.restore_window_s` (60) |
| Almacenamiento y registro | `storage.retry_s` (60), `logging.frontend_max_per_min` (60), `logging.detailed_hours` (24) |
| Colector | `collector.max_message_bytes` (1 MiB), `collector.max_invalid_messages` (3), `collector.max_restarts` (3), `collector.restart_window_min` (10), `collector.stall_intervals` (3), `collector.parent_check_s` (2) |

**Límite térmico efectivo**: Intel, `TjMax − TCC offset` leídos de `MSR_TEMPERATURE_TARGET`; AMD, límite de Tctl por familia según la tabla versionada `thermal-limits-v1` (por ejemplo, 95 °C en Zen 4/Zen 5 de sobremesa) o el que exponga el colector. Sin límite conocido se usan umbrales absolutos y el anillo indica escala aproximada.

**Estado térmico**

| Condición | Con límite efectivo conocido | Sin límite conocido |
|---|---|---|
| Normal | margen > 8 °C | < 85 °C |
| Temperatura alta | margen ≤ 8 °C | ≥ 85 °C |
| En el límite / crítica | margen ≤ 3 °C | ≥ 95 °C |

Ningún estado térmico implica por sí solo limitación.

**Temperatura representativa**: prioridad `package`/`Tdie` directo → máximo de núcleos → `Tctl` con offset conocido → `Tctl` sin offset (calidad `substitute`). En AMD, si coexisten `Tctl` y `Tdie`, se prefiere `Tdie`. La tarjeta indica cuál se usa.

**Niveles de cobertura**

| Nivel | Señales disponibles | Qué puede concluir | Confianza máxima |
|---|---|---|---|
| A — completo | temperatura, frecuencia activa, carga por núcleo, potencia de paquete, límite de potencia efectivo (PL1/PL2/Tau) y razones de limitación (`THERMAL`, `PROCHOT`, `PL1`, `PL2`, `EDP`, corriente) | todas las clasificaciones, gravedad y potencial cuantificado | alta |
| B — con potencia | temperatura, frecuencia activa, carga por núcleo y potencia de paquete | térmica/potencia/equipo **probables** por mesetas (la mixta exige nivel A); sin cifra de potencial | media |
| C — básico | temperatura, frecuencia activa y carga por núcleo | solo «meseta en el límite con frecuencia activa por debajo de la base» como térmica probable; el resto, indeterminado | baja |

En Intel el nivel A requiere el acceso avanzado. En AMD de consumo no existe un registro documentado de razones de limitación: se usa la tabla PM del SMU solo para versiones incluidas en una lista permitida y versionada; fuera de ella el techo es el nivel B.

**Carga sostenida y frecuencia activa**: un procesador lógico está activo si su utilidad es ≥ 80 % en la muestra. Hay carga sostenida cuando al menos un núcleo físico permanece activo durante toda la ventana. La frecuencia activa de un grupo es la mediana de la frecuencia activa de sus núcleos activos. La carga total no es condición.

**Ventanas**: instantánea 1–3 s (solo visualización); estable 60 s (decisión), deslizante cada 10 s; sesión (tendencias del límite de potencia).

**Ventana de turbo**: tras cada inicio de carga (paso de < 30 % a carga sostenida) se excluyen de la decisión sostenida los primeros `max(Tau, 60 s)`, con Tau leído de `MSR_PKG_POWER_LIMIT` cuando esté disponible. Un escalón de potencia ≥ 15 % a la baja en ≤ 5 s con carga constante se registra como evento `turbo_end` («fin del turbo de potencia, esperado»).

**Mesetas** (en la ventana estable, fuera de la ventana de turbo):
- Meseta térmica: margen ≤ 3 °C y desviación típica de la temperatura ≤ 1,5 °C.
- Meseta de potencia: coeficiente de variación de la potencia de paquete ≤ 3 %. Por sí sola no indica un límite de potencia: en equilibrio térmico la potencia también se estabiliza, por eso las reglas que la usan exigen además margen térmico.

**Ocupación de razones**: fracción de muestras de la ventana estable en que el bit de registro (log) de la razón estaba activo; los bits se limpian tras cada lectura.

**Reglas de clasificación** (se evalúan en este orden; la primera que se cumple decide):

| # | Clasificación | Nivel A (razones directas) | Niveles B/C (inferencia) |
|---|---|---|---|
| 1 | `indeterminate` | falta temperatura o frecuencia activa | ídem |
| 2 | `hot_unproven` / `normal` | sin carga sostenida, o dentro de la ventana de turbo: `hot_unproven` si la temperatura es alta, si no `normal` | ídem |
| 3 | `mixed_limit` | ocupación `THERMAL` ≥ 20 % y ocupación de potencia (`PL1`/`PL2`/`EDP`/corriente) ≥ 20 % | — nunca (sin límite de potencia medido no puede demostrarse) |
| 4 | `thermal_confirmed` | ocupación `THERMAL` ≥ 20 % | — nunca |
| 5 | `thermal_probable` | — | B: meseta térmica (causa alternativa listada: «posible límite de potencia simultáneo, no medible en este nivel»). C: meseta térmica con frecuencia activa < base |
| 6 | `platform_limited` · `external_prochot` | ocupación `PROCHOT` ≥ 20 % sin `THERMAL` | — |
| 7 | `platform_limited` · `chassis_thermal` | el límite de potencia efectivo baja ≥ 10 % durante la sesión sin cambio de plan ni de alimentación, con margen > 8 °C, de forma progresiva (≥ 2 escalones o descenso repartido en ≥ 3 min) | B: el nivel de la meseta de potencia baja ≥ 15 % a lo largo de la sesión con carga y contexto constantes y margen > 8 °C, de forma progresiva |
| 7b | `indeterminate` (causa alternativa `oem_mode_change`) | el límite de potencia efectivo baja ≥ 10 % en un único escalón de ≤ 5 s sin cambio de plan ni de alimentación; no se recomienda ventilar y se registra el evento informativo `oem_mode_change` | B: ídem sobre el nivel de la meseta de potencia (≥ 15 %) |
| 8 | `power_limited` | ocupación de potencia ≥ 20 % con margen > 3 °C | B: meseta de potencia con margen > 8 °C |
| 9 | `indeterminate` | frecuencia activa ≥ 8 % por debajo de la de la ventana de turbo sin meseta ni razón; causas alternativas: gestión de energía de Windows (EcoQoS, EPP, modo eficiencia), plan energético | ídem |
| 10 | `normal` | carga sostenida sin nada de lo anterior | ídem |

**Gravedad** (reglas 3–8, salvo 7b): `below_base` si la frecuencia activa de los núcleos activos es < 97 % de su frecuencia base (`Processor Frequency`) durante ≥ 30 s de la ventana; si no, `boost`. `boost` se comunica como «limitado por encima de la frecuencia garantizada: dentro de especificación»; `below_base`, como «por debajo de la frecuencia garantizada».

**Clasificación de una sesión** (informe congelado): las ventanas estables se evalúan una a una y las consecutivas de la misma clase se fusionan en `limit_event`. La clase del informe es:
1. `indeterminate` si más del 50 % del tiempo con carga sostenida quedó `indeterminate`;
2. si no, la clase limitante (reglas 3–8) con mayor tiempo acumulado, siempre que sume ≥ 60 s y ≥ 10 % del tiempo con carga sostenida; en empate decide `below_base` y después el orden de la tabla;
3. si no, `hot_unproven` si hubo ventanas con temperatura alta; en otro caso `normal`.

La gravedad del informe es `below_base` si esa clase acumuló ≥ 30 s en `below_base`. El informe lista todas las clases con su duración y el intervalo analizado (FR-010).

**Confianza**: puntuación 0..1 de **solidez de la evidencia** (no es una probabilidad) a partir de nivel de cobertura, calidad de sensores, duración y estabilidad de la ventana; bandas `baja` < 0,45 ≤ `media` < 0,75 ≤ `alta`; techos según el nivel de cobertura (FR-070). Los pesos se calibran con el corpus etiquetado antes de publicar.

**Potencial con mejor refrigeración** (reglas 3, 4, 5 y 7, fuera de la ventana de turbo):
- Método «techo de potencia»: `g = (PL1_ref / P)^(1/3) − 1`, donde P es la potencia de paquete medida y PL1_ref es:
  - clases térmica y mixta: el PL1 efectivo actual;
  - `chassis_thermal`: el PL1 efectivo máximo observado en la sesión fuera de la ventana de turbo con el mismo contexto energético (lo que el equipo recuperaría con mejor ventilación).
- Acotación: `g ≤ f_turbo / f_activa − 1`, donde `f_turbo` es la mediana de la frecuencia activa en la ventana de turbo de la misma sesión con el mismo número de núcleos activos y `f_activa` la de la ventana estable. El potencial de rendimiento se expresa como `[0,5·g, 1,0·g]`: el extremo inferior cubre cargas limitadas por memoria y la curva tensión-frecuencia.
- Tramos: `< 3 %` «apenas mejoraría», `3–10 %` «mejora moderada», `> 10 %` «mejora notable». El rango se redondea hacia fuera a múltiplos de 5 %.
- Sin PL1 conocido: sin cifra; si la gravedad es `below_base`, tramo cualitativo «probablemente notable» con confianza baja.
- `power_limited`: «la refrigeración apenas influye»; sin cifra.

**Rendimiento medido (diagnóstico guiado)**: el generador cuenta operaciones completadas por hilo y segundo. El informe muestra la mediana de los últimos 120 s de la carga sostenida relativa a la de los primeros 20 s de carga, y desglosa la diferencia según la ocupación de cada causa en la fase sostenida (fin del turbo, potencia, térmica, equipo). Es la única cifra de «rendimiento» en porcentaje de la aplicación y se rotula «rendimiento de la carga de prueba».

**Comparación antes/después**: solo entre diagnósticos guiados completados con el mismo perfil de duración, el mismo contexto energético y la misma versión del generador; compara el rendimiento medido sostenido. No existen baselines aprendidos.

**Perfiles de muestreo**

| Perfil | Intervalo | Detalle solicitado | Persistencia por núcleo |
|---|---:|---|---|
| Bajo consumo | 5 s | representativo | no |
| Normal | 1 s | por grupo | no (salvo `sampling.per_core_history`) |
| Diagnóstico | 500 ms | por núcleo | sí |

El detalle por núcleo se mantiene siempre en memoria para la pantalla CPU; el histórico se agrega por grupo salvo en sesiones guiadas o en perfil Diagnóstico.

**Alertas**: tipos `thermal_confirmed`, `power_limited`, `platform_limited`, `collector_lost` y `guided_finished`; `update_available` pertenece al actualizador. Las alertas de limitación exigen gravedad `below_base` **y** `certainty = observed` (razones directas, nivel A): una clase inferida por mesetas (niveles B/C, incluida `thermal_probable`) nunca genera notificación; se comunica solo en la ventana y en el icono de bandeja (`aviso`). Persistencia mínima 90 s; enfriamiento 30 min por tipo; periodo de silencio opcional (sin valor inicial). Una alerta de limitación abre `Análisis` centrado en el evento.

**Prueba guiada**: comprobación ≤ 30 s; reposo opcional 60 s (omitible); calentamiento progresivo 90 s; carga sostenida 180 s (`corta`), 240 s (`estándar`) o 360 s (`larga`), de modo que los últimos 120 s quedan siempre fuera de la ventana de turbo; recuperación 120 s. Parada automática (FR-085): temperatura > límite efectivo + 2 °C durante 3 muestras; temperatura en el límite con frecuencia activa < 50 % de la base durante 10 s; 3 muestras consecutivas sin sensor crítico; generador sin respuesta durante 5 s. Alcanzar el límite térmico **no** detiene la prueba. Ocultar la ventana o suspender el equipo cancela la prueba. Atajo de parada: `Ctrl+Shift+X`.

**Ventana**: tamaño mínimo 480×600 px lógicos; si la altura útil del monitor es menor (por ejemplo, 1080p al 200 %), el mínimo de altura pasa a esa altura útil, nunca por debajo de 500 px lógicos. Con altura útil inferior a 500 px lógicos, la ventana se abre maximizada y el contenido se desplaza verticalmente, con barra de título, banner global y `Detener ahora` fijos. Tamaño inicial 1100×760 centrado en la pantalla principal, reducido al área útil si no cabe.

**Vidrio (degradación automática)** (`glass.*`): el nivel efectivo baja de `full` a `reduced` cuando la tasa de fotogramas de la WebView es < 50 fps sostenidos durante 3 s (`glass.degrade_fps`, `glass.degrade_window_s`) o cuando la CPU atribuible a composición en reposo supera el 1 % durante 30 s (`glass.degrade_idle_cpu_pct`, `glass.degrade_idle_window_s`). Vuelve al nivel preferido solo tras 60 s por encima de 55 fps (`glass.restore_fps`, `glass.restore_window_s`: histéresis). La degradación no modifica `appearance.glass`; el estado degradado es visible en Ajustes › Apariencia. Valores provisionales hasta el spike de vidrio (T019c); su cambio exige nueva versión de reglas.

**Atajos**: `Ctrl+1`…`Ctrl+6` navegación en el orden de la barra lateral; `Ctrl+,` Ajustes; `Ctrl+Shift+X` detener prueba; `Ctrl+E` exportar; `F1` ayuda; `Esc` cierra diálogos y tooltips.

**Cierre durante operaciones**: con prueba activa se pregunta «Detener y salir / Cancelar»; con exportación se espera ≤ 5 s y se cancela; con descarga se cancela y descarta; durante instalación no se puede cerrar.

**Exportación**: CSV en formato largo (`timestamp_utc, monotonic_ms, sensor_id, metric, scope, value, status, quality`), separador coma, punto decimal, UTF-8 con BOM, tiempos UTC ISO-8601, nombre `throttlewatch_<tipo>_<fecha>_<id-corto>.<ext>`. La anonimización conserva fabricante, modelo comercial, topología y versiones; elimina nombre de equipo, usuario, números de serie, direcciones MAC, rutas, huella de monitores y GUID de plan energético.

**Almacenamiento**: datos en `%LOCALAPPDATA%\ThrottleWatch`; registros en `logs/` en JSON por líneas (UTC), rotación de 5 ficheros × 5 MB y nivel `info`; `Registro detallado` sube a `debug` durante 24 h o hasta reiniciar (FR-086). La interfaz reenvía al backend solo `warn` y `error` (y `debug` con registro detallado), con un máximo de 60 eventos por minuto. Los registros se borran con `Eliminar todos mis datos`, con `Restablecer` y al salir con retención `solo sesión`. Disco lleno: modo solo memoria con aviso y reintento cada 60 s. Base de datos dañada: se renombra a `.corrupt-<fecha>`, se crea una nueva y se ofrece exportar la dañada.

**Colector (supervisión)** (`collector.*`; los valores normativos del protocolo están en `contracts/ipc-protocol.md`): tamaño máximo por mensaje 1 MiB (`collector.max_message_bytes`); cierre y reinicio tras 3 mensajes inválidos consecutivos (`collector.max_invalid_messages`); como máximo 3 reinicios con retroceso en una ventana de 10 min (`collector.max_restarts`, `collector.restart_window_min`), después estado `failed` con intervención del usuario; sidecar bloqueado si faltan 3 intervalos de muestra (`collector.stall_intervals`); comprobación del proceso padre cada 2 s (`collector.parent_check_s`).

### Entidades clave

- **Dispositivo CPU**: identidad normalizada, fabricante, familia, topología y capacidades.
- **Descriptor de sensor**: sensor original, magnitud normalizada, unidad, alcance y calidad.
- **Muestra**: valores coincidentes en un instante monotónico y su estado de validez.
- **Sesión**: intervalo de monitorización o prueba, contexto energético, historial de nivel de cobertura (cambios con marca de tiempo, FR-090) y resultado.
- **Contexto energético**: fuente de alimentación, plan energético activo y batería, registrados con cada muestra.
- **Referencia guiada**: resultado medido de un diagnóstico guiado marcado por el usuario, usado solo para comparaciones «antes/después».
- **Evento de limitación**: tipo, inicio, fin, severidad y evidencias.
- **Diagnóstico**: clasificación, confianza, estimación opcional y explicación.
- **Preferencias**: onboarding, muestreo, retención, alertas, privacidad, idioma, apariencia, ciclo de vida, ventana, inicio con Windows y actualizaciones.
- **Estado de actualización**: habilitación, última comprobación automática, versión disponible, descarga verificada, progreso y último error.

## Criterios de éxito medibles

- **SC-001**: En pruebas moderadas con personas (no automatizables en CI), al menos el 90 % de nuevos usuarios identifica correctamente el estado principal en menos de 10 s.
- **SC-002**: El 100 % de las cifras de potencial o rendimiento muestra método, entradas usadas (límite de potencia o carga de prueba), tramo, rango redondeado y confianza; ninguna cifra aparece sin nivel A o medición guiada.
- **SC-003**: Ninguna traza “caliente sin limitación” ni “fin de turbo” del corpus se etiqueta como limitación térmica confirmada o probable.
- **SC-004**: Al menos el 95 % de las trazas de nivel A con ocupación `THERMAL` ≥ 20 % se clasifica como térmica confirmada, y ninguna traza con solo `PROCHOT` lo hace.
- **SC-005**: En trazas de nivel A, el motor acierta la clase (térmica, potencia, equipo, mixta, normal) en al menos el 90 % de las ventanas etiquetadas.
- **SC-016**: Con las mismas trazas degradadas artificialmente a nivel B (eliminando razones, límites y TCC offset), el motor coincide con la etiqueta de nivel A en al menos el 80 % de las ventanas en que emite una causa, según esta equivalencia: `thermal_confirmed` → `thermal_probable` y `mixed_limit` → `thermal_probable` cuentan como acierto; `power_limited` → `power_limited` y `platform_limited` → `platform_limited` cuentan como acierto; `indeterminate` no cuenta. Ninguna ventana etiquetada térmica o mixta se clasifica `power_limited` con confianza `media` (inversión térmica → potencia), y ninguna etiquetada potencia se clasifica `thermal_probable` con confianza `media`.
- **SC-017**: Al menos el 80 % de las trazas etiquetadas como gestión térmica del fabricante se clasifican `platform_limited · chassis_thermal`, y ninguna recibe la recomendación de no mejorar la refrigeración.
- **SC-018**: La gravedad (`boost` / `below_base`) coincide con la etiqueta en al menos el 95 % de las ventanas limitadas del corpus.
- **SC-019**: Instalar el acceso avanzado en un equipo sin PawnIO produce exactamente una petición UAC; los 10 arranques siguientes de la aplicación (con inicio con Windows activado) producen cero peticiones UAC y alcanzan el nivel A en menos de 10 s desde el arranque del sidecar.
- **SC-020**: Al detener el servicio PawnIO durante una sesión pasiva, no se pierde ninguna muestra, el cambio de nivel queda registrado con marca de tiempo dentro de un intervalo de muestreo y la cobertura muestra el aviso con la acción de reparar en menos de 5 s; no se emite `collector_lost`.
- **SC-006**: El panel mantiene los presupuestos de consumo y latencia definidos en NFR-001 a NFR-003 durante una sesión de una hora.
- **SC-007**: Todas las funciones principales son utilizables con teclado y en modo de movimiento reducido.
- **SC-008**: Una exportación anonimizada supera una prueba automática que busca identificadores excluidos.
- **SC-009**: Intel moderno, AMD moderno y al menos dos generaciones anteriores de cada fabricante completan el flujo pasivo cuando el hardware expone sensores suficientes.
- **SC-010**: La desconexión simulada del colector conserva la UI, explica el fallo y recupera el muestreo o guía al usuario.
- **SC-011**: El 100 % de las rutas del primer inicio puede completarse u omitirse con teclado, y una interrupción reanuda la última diapositiva alcanzada.
- **SC-012**: Todos los locales ibéricos especificados resuelven a español y una muestra de locales no ibéricos, incluido portugués, resuelve a inglés.
- **SC-013**: Tras mover o redimensionar la ventana y reiniciar, la geometría se restaura dentro del área visible en todos los escenarios de uno o varios monitores probados.
- **SC-014**: Con actualizaciones desactivadas, el 100 % de las sesiones no produce tráfico del actualizador; ningún artefacto sin firma válida alcanza el estado instalable.
- **SC-015**: El 100 % de las pantallas incluidas en una entrega supera la matriz visual y funcional de ambos idiomas, ambos temas y los tamaños compacto, medio y expandido, sin pérdida de acciones esenciales, desbordamiento que impida su uso ni desviaciones no documentadas del sistema visual aprobado.

## Suposiciones

- El producto inicial es para Windows y uso local individual.
- LibreHardwareMonitor cubre gran parte del hardware, pero no garantiza todos los sensores en todos los equipos.
- La disponibilidad de PawnIO o equivalente se trata como una capacidad detectable, no como certeza. PawnIO es un controlador compartido por máquina que otras aplicaciones pueden haber instalado o necesitar (FR-089).
- El usuario puede aceptar elevación durante instalación/reparación, pero la interfaz diaria no la necesita.
- La referencia local es más honesta que una base de datos universal de puntuaciones.
- El nombre definitivo es `ThrottleWatch`; el icono, identidad visual final y modelo de distribución se cerrarán antes del empaquetado público.
- Activar el actualizador constituye consentimiento para su aviso nativo de versión disponible; el interruptor general de notificaciones controla alertas térmicas, no ese aviso solicitado por separado.
- Un perfil de datos por usuario de Windows; no hay sincronización entre usuarios ni equipos.
- Una única CPU física (un socket); se toma la primera si el sistema expone varias.
- La temperatura se expresa solo en grados Celsius.
- La interfaz de ThrottleWatch no se ejecuta como servicio de Windows; en bandeja sigue siendo un proceso del usuario. Solo el sidecar puede ejecutarse elevado mediante el lanzador de FR-088.
- El sidecar se ejecuta con los mismos privilegios que la interfaz (sin elevación) mientras no exista acceso avanzado; con acceso avanzado instalado se ejecuta elevado mediante el lanzador de FR-088, documentado como excepción en su ADR.
- Las máquinas virtuales no son un caso principal: se soportan solo con cobertura reducida y estado explícito.
- `LibreHardwareMonitorLib` no expone la clase de núcleo (P/E/LP); el sidecar la deriva de `GetLogicalProcessorInformationEx` y CPUID (hoja 0x1A en Intel). Cuando no puede, los grupos se marcan `unknown`.
- Es probable que `LibreHardwareMonitorLib` no exponga frecuencia activa ni razones de limitación; el diseño asume que la frecuencia activa se deriva de los contadores de Windows y que el nivel A (razones, límites de potencia, TCC offset) requiere el acceso avanzado.
- **El valor diferencial del producto depende del nivel A.** Sin acceso avanzado la aplicación sigue siendo útil como explicación prudente (niveles B y C), pero no confirma causas ni cuantifica. Por eso la fase 0 incluye una puerta de viabilidad (véase `plan.md`).
- La relación entre frecuencia y potencia se modela como cúbica (P ∝ f·V², con V aproximadamente proporcional a f); es una aproximación de primer orden que el rango `[0,5·g, 1,0·g]` absorbe y que se contrasta con la medición guiada.
- El idioma del sistema se toma del primer idioma de visualización de Windows; el tema del sistema, del modo de aplicación (`AppsUseLightTheme`).
- La documentación esencial se empaqueta con la aplicación; «Documentación en línea» abre el navegador del sistema por gesto explícito del usuario y es la única otra acción con red además del actualizador.
- El instalador es por usuario y sin elevación; el acceso de bajo nivel, si el spike lo aprueba, se instala en un paso separado por máquina ejecutando el instalador oficial de PawnIO empaquetado (FR-087). Desinstalar pregunta si conservar los datos.

## Fuera de alcance del MVP

- GPU, SSD, placa base y ventiladores como objeto principal del diagnóstico.
- Control de ventiladores, undervolt, overclock, cambios PL1/PL2/PPT o plan energético.
- Comparativas públicas entre modelos de CPU.
- Diagnóstico remoto centralizado o panel empresarial.
- macOS y Linux.
- Prometer causalidad o ganancia exacta cuando el hardware no aporta evidencia suficiente.
- Telemetría o envío de informes de fallo por red, incluso anónimos y opt-in.
- Canales de actualización (beta, nightly).
- Sistemas multi-socket.
- Detección de modos OEM (silencioso, rendimiento) como factor de confusión explícito.
- Unidades de temperatura distintas de Celsius.
