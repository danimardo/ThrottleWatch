# Especificación de funcionalidad: diagnóstico térmico de CPU

**Rama**: `001-cpu-thermal-diagnostics`  
**Creada**: 2026-09-17  
**Estado**: Lista para planificación  
**Entrada**: Utilidad Windows visualmente atractiva que mida CPU Intel/AMD, detecte limitación térmica y explique cuánto rendimiento potencial se pierde sin confundir calor con otras limitaciones.

## Objetivo

ThrottleWatch ayuda a responder tres preguntas:

1. ¿Mi CPU está demasiado caliente en este momento o durante una carga sostenida?
2. ¿El calor está reduciendo realmente su rendimiento?
3. ¿Mejorar la refrigeración probablemente ayudaría, o el límite principal es otro?

La utilidad debe convertir sensores técnicos en una explicación verificable, conservar el detalle para usuarios avanzados y abstenerse de cuantificar cuando no exista evidencia suficiente.

## Escenarios de usuario y pruebas

### Historia 1 — Comprender el estado térmico actual (Prioridad P1)

Como usuario quiero abrir la aplicación y entender en pocos segundos la temperatura, carga, frecuencia efectiva y estado térmico de mi CPU.

**Por qué es prioritaria:** es el valor mínimo del producto y permite validar la cobertura de sensores antes de inferir causas.

**Prueba independiente:** con una fuente de sensores compatible, el panel muestra datos actuales, tendencia y estado; con sensores incompletos, muestra claramente la cobertura reducida.

**Escenarios de aceptación:**

1. **Dado** un equipo con sensores de CPU disponibles, **cuando** se abre el panel, **entonces** aparecen modelo, temperatura representativa, carga, reloj efectivo o sustituto, potencia cuando exista y antigüedad de la última muestra.
2. **Dado** un sensor ausente, **cuando** se muestra su tarjeta, **entonces** figura como “No disponible” con explicación y nunca como `0`.
3. **Dado** que la temperatura se acerca al límite conocido, **cuando** se actualiza la muestra, **entonces** el panel muestra la distancia al límite y cambia de estado usando color, icono y texto.
4. **Dado** un procesador híbrido, **cuando** existen grupos P/E/LP, **entonces** la aplicación no mezcla sus relojes en una media sin ponderar ni ocultar la topología.

---

### Historia 2 — Detectar y explicar una limitación (Prioridad P1)

Como usuario quiero saber si la CPU reduce su capacidad por temperatura, potencia u otra causa para tomar una decisión correcta.

**Por qué es prioritaria:** diferencia el producto de un simple termómetro.

**Prueba independiente:** al reproducir trazas conocidas, el motor clasifica correctamente limitación térmica confirmada, probable, por potencia, mixta, sin limitación o indeterminada y enumera las evidencias.

**Escenarios de aceptación:**

1. **Dado** un indicador directo de thermal throttling activo bajo carga, **cuando** persiste durante la ventana configurada, **entonces** el diagnóstico es “Limitación térmica confirmada”.
2. **Dado** calor próximo al límite y caída correlacionada del reloj efectivo bajo carga comparable, pero sin indicador directo, **cuando** la evidencia supera el umbral, **entonces** el diagnóstico es “Evidencia compatible con limitación térmica”, no “confirmada”.
3. **Dado** reloj reducido, margen térmico holgado e indicador de límite de potencia persistente, **cuando** se diagnostica, **entonces** la causa principal es potencia y no se recomienda mejorar la refrigeración como primera medida.
4. **Dado** temperatura elevada sin carga o sin referencia de frecuencia, **cuando** se diagnostica, **entonces** se advierte de la temperatura pero no se afirma pérdida de rendimiento.
5. **Dado** que concurren límites térmicos y eléctricos, **cuando** ambos tienen evidencia material, **entonces** el resultado se muestra como causa mixta y no fuerza una causa única.

---

### Historia 3 — Estimar rendimiento disponible (Prioridad P1)

Como usuario quiero una estimación honesta del rendimiento que conservo y del que podría recuperar con mejor refrigeración.

**Por qué es prioritaria:** responde a la pregunta principal del producto, pero necesita límites estrictos para no dar falsa precisión.

**Prueba independiente:** usando una traza con referencia local comparable, se calcula rango, confianza y método; sin referencia, el porcentaje se omite.

**Escenarios de aceptación:**

1. **Dado** un baseline local válido y una carga comparable, **cuando** el reloj efectivo ponderado cae de forma sostenida por evidencia térmica, **entonces** se muestra un intervalo estimado de rendimiento disponible y pérdida potencial.
2. **Dado** que no existe baseline comparable, **cuando** hay calor o throttling, **entonces** se muestra la limitación y su duración, pero no un porcentaje inventado.
3. **Dado** un procesador híbrido, **cuando** se estima rendimiento, **entonces** el cálculo se realiza por grupos de núcleos y pondera solo grupos activos.
4. **Dado** que el resultado estimado es 81,4 %, **cuando** se presenta al usuario, **entonces** se redondea y/o expresa como rango razonable, por ejemplo `~78–84 %`, con nivel de confianza.

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
2. **Dado** un término como *thermal throttling*, TjMax o baseline, **cuando** aparece en el recorrido, **entonces** incluye una explicación breve en lenguaje corriente.
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

---

### Historia 10 — Configurar el comportamiento sin perder el control (Prioridad P2)

Como usuario quiero configurar monitorización, bandeja, avisos, privacidad y datos con opciones comprensibles, para adaptar ThrottleWatch sin tener que conocer frecuencias o detalles internos.

**Por qué es prioritaria:** varias capacidades consumen recursos, conservan datos o continúan tras cerrar la ventana y deben depender de decisiones visibles.

**Prueba independiente:** se cambian ajustes, se reinicia y se comprueba su persistencia, sus dependencias y el efecto de borrar datos o restaurar valores de fábrica.

**Escenarios de aceptación:**

1. **Dado** que no existe una acción de cierre guardada, **cuando** se pulsa la X por primera vez, **entonces** se pregunta entre salir y continuar en la bandeja midiendo, se guarda la elección y se indica dónde cambiarla; elegir bandeja activa la monitorización en segundo plano; cerrar el diálogo con `Esc` no guarda nada ni cierra la ventana.
7. **Dado** que la monitorización en bandeja está habilitada y la acción de cierre es `bandeja`, **cuando** el usuario deshabilita la monitorización en bandeja, **entonces** la acción de cierre pasa a `salir` en la misma operación y la interfaz lo comunica.
2. **Dado** `Iniciar con Windows` desactivado por defecto, **cuando** el usuario lo activa, **entonces** puede elegir iniciar mostrando la ventana o, si la monitorización de bandeja está habilitada, iniciar oculto.
3. **Dado** un usuario no técnico, **cuando** configura el muestreo, **entonces** elige entre `Bajo consumo`, `Normal` y `Diagnóstico`, dejando los valores numéricos en ajustes avanzados.
4. **Dado** una instalación nueva, **cuando** se consultan notificaciones y retención, **entonces** los avisos están desactivados y la retención es de siete días; también existen `solo sesión`, `1 día` y `30 días`.
5. **Dado** que el usuario elimina todos sus datos, **cuando** confirma, **entonces** se borran sesiones, muestras, informes y baselines, pero se conservan preferencias y estado del onboarding.
6. **Dado** que el usuario restablece ThrottleWatch, **cuando** confirma una advertencia diferenciada, **entonces** se eliminan datos y preferencias y el siguiente inicio se comporta como una instalación nueva.

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
- Segunda instancia de la aplicación: se enfoca la existente y la nueva termina.
- Temperatura disponible solo como paquete, solo por núcleo, `Tctl` con offset o margen respecto a TjMax.
- TjMax desconocido o dinámico; sensores que devuelven valores imposibles, congelados o intermitentes.
- Cambio de corriente alterna a batería, plan energético o modo OEM durante una sesión.
- Suspensión, hibernación, reanudación, cambio de hora o salto del reloj del sistema.
- Topología híbrida, SMT, núcleos aparcados o cambios de afinidad.
- Carga parcial, variable o concentrada en pocos núcleos.
- Sidecar detenido, protocolo incompatible, permisos insuficientes o controlador ausente.
- Dos instancias de la aplicación o una actualización durante el muestreo.
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
- **FR-003**: El sistema DEBE obtener, cuando estén disponibles, carga, temperatura, reloj efectivo, reloj activo, potencia de paquete, margen térmico e indicadores de límites térmicos/eléctricos.
- **FR-004**: El sistema DEBE normalizar sensores Intel y AMD a un modelo común sin perder el nombre original.
- **FR-005**: El sistema DEBE muestrear a 1 Hz por defecto y permitir perfiles de bajo consumo y diagnóstico.
- **FR-006**: El sistema DEBE conservar un búfer reciente en memoria y un historial local sujeto a retención configurable.
- **FR-007**: El sistema DEBE representar valores ausentes como desconocidos, nunca como cero.
- **FR-008**: El sistema DEBE calcular estados térmicos mediante margen al límite cuando exista y mediante reglas conservadoras cuando no exista.
- **FR-009**: El motor DEBE clasificar: normal, temperatura alta sin limitación demostrada, térmica probable, térmica confirmada, potencia/corriente, mixta e indeterminada.
- **FR-010**: Todo diagnóstico DEBE incluir evidencias favorables, factores alternativos y confianza.
- **FR-011**: El sistema NO DEBE interpretar el porcentaje de tiempo con throttling como porcentaje de rendimiento perdido.
- **FR-012**: El sistema NO DEBE usar el turbo máximo anunciado como referencia de rendimiento multinúcleo sostenido.
- **FR-013**: La estimación de rendimiento DEBE usar un baseline local de carga y topología comparables.
- **FR-014**: Si la referencia no es válida, el sistema DEBE omitir el porcentaje y explicar qué falta.
- **FR-015**: La estimación DEBE mostrar un rango y nivel de confianza, no precisión decimal engañosa.
- **FR-016**: El sistema DEBE separar grupos de núcleos heterogéneos y ponderar solo grupos activos.
- **FR-017**: La aplicación DEBE ofrecer una prueba guiada opcional, cancelable y con límites de parada.
- **FR-018**: La prueba NO DEBE modificar configuración de firmware, voltajes, potencia ni ventiladores.
- **FR-019**: El panel principal DEBE mostrar estado, temperatura, margen térmico, carga, reloj efectivo, potencia disponible y resumen de diagnóstico.
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
- **FR-031**: El sistema DEBE registrar fallos del colector y del diagnóstico sin incluir datos sensibles innecesarios.
- **FR-032**: Preferencias, baseline y reglas aplicadas DEBEN conservar su versión para interpretar diagnósticos históricos.
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
- **FR-043b**: El efecto de vidrio de la interfaz DEBE ofrecer `Sistema`, `Completo`, `Reducido` y `Sin`; `Sistema` DEBE seguir el ajuste «Efectos de transparencia» de Windows. La aplicación DEBE degradar el nivel automáticamente cuando la tasa de fotogramas no alcance el presupuesto y DEBE mantener el contraste AA del texto en todos los niveles.
- **FR-044**: La ventana principal DEBE usar una barra de título propia con icono, nombre, región de arrastre, minimizar, maximizar/restaurar, cerrar y doble clic para maximizar/restaurar, disponible también en onboarding y errores iniciales.
- **FR-045**: La aplicación DEBE conservar posición, tamaño, estado maximizado y último tamaño restaurado; nunca DEBE restaurar la ventana completamente fuera del área visible ni recordar un estado minimizado.
- **FR-046**: En el primer cierre sin decisión guardada, el sistema DEBE preguntar si sale o continúa en la bandeja, guardar obligatoriamente la elección e indicar que puede cambiarse en Ajustes. Elegir bandeja DEBE activar `tray.monitoring_enabled`; no existe un estado «en bandeja sin muestreo». Cerrar el diálogo sin elegir DEBE dejar la decisión sin guardar y la ventana abierta.
- **FR-047**: `Iniciar con Windows` DEBE estar desactivado por defecto y, al activarse, permitir mostrar la ventana o iniciar oculto; iniciar oculto solo DEBE permitirse si la monitorización en bandeja está habilitada.
- **FR-048**: Los avisos DEBEN estar desactivados por defecto; el onboarding solo DEBE informar de que pueden habilitarse en Ajustes.
- **FR-049**: La configuración de muestreo DEBE presentar los perfiles `Bajo consumo`, `Normal` y `Diagnóstico`; sus frecuencias concretas solo aparecen en ajustes avanzados.
- **FR-050**: `Eliminar todos mis datos` DEBE borrar sesiones, muestras, eventos, informes y baselines tras confirmación, conservando preferencias y estado del onboarding.
- **FR-051**: `Restablecer ThrottleWatch` DEBE requerir una confirmación diferenciada, borrar datos y preferencias y provocar un primer inicio limpio.
- **FR-052**: La comprobación de actualizaciones DEBE estar desactivada por defecto y, mientras lo esté, NO DEBE realizar tráfico de red.
- **FR-053**: Con actualizaciones activas, el sistema DEBE comprobar automáticamente como máximo una vez cada 24 horas y ofrecer una comprobación manual que pueda reintentar antes de ese plazo.
- **FR-054**: Detectar una versión, descargarla e instalarla DEBEN ser tres gestos separados; ningún paso DEBE iniciar automáticamente el siguiente.
- **FR-055**: El instalador descargado DEBE validarse con firma Ed25519 antes de ofrecer instalación; un artefacto parcial o inválido DEBE descartarse.
- **FR-056**: Una actualización disponible DEBE aparecer en Ajustes y como notificación nativa; la descarga DEBE mostrar progreso.
- **FR-057**: Instalar DEBE exigir confirmación, avisar del cierre y quedar bloqueado mientras haya un diagnóstico guiado, exportación, importación o borrado/restablecimiento en curso, mostrando el motivo del bloqueo.
- **FR-058**: El actualizador DEBE usar un endpoint fijo de GitHub Releases, ejecutarse desde el backend, no enviar identificadores ni telemetría y aislar todos sus fallos del resto de la aplicación. No existe selección de canal: solo se publican versiones estables.
- **FR-059**: El icono de bandeja DEBE distinguir cinco estados: `normal`, `aviso`, `crítico`, `desconocido` y `desconectado`, mapeados desde la clasificación en vivo y el estado del colector. Un clic DEBE mostrar u ocultar la ventana; el menú contextual DEBE ofrecer estado actual, abrir, pausar/reanudar muestreo y salir.
- **FR-060**: El comportamiento en batería DEBE ser configurable mediante `sampling.on_battery` con valores `mantener`, `bajo consumo` y `pausar`; el valor inicial es `mantener`. Un cambio de fuente de alimentación DEBE aplicarse en la siguiente muestra.
- **FR-061**: Deshabilitar la monitorización en bandeja DEBE corregir atómicamente cualquier configuración dependiente incompatible (`startup.mode=tray` → `window`, `lifecycle.close_action=tray` → `exit`) y comunicarlo en la respuesta.
- **FR-062**: Un control dependiente de otro ajuste DEBE permanecer visible, deshabilitado y con una explicación de la condición que lo habilita; nunca DEBE ocultarse.
- **FR-063**: `Eliminar todos mis datos` y `Restablecer ThrottleWatch` DEBEN ejecutarse como transacciones; un fallo parcial DEBE revertir lo posible y comunicar qué no pudo completarse.
- **FR-064**: El mapa de núcleos DEBE permitir alternar la magnitud representada entre temperatura y reloj sin perder la selección de núcleo.
- **FR-065**: Tras una reanudación desde suspensión o hibernación, el sistema DEBE marcar el hueco, repetir el descubrimiento de capacidades y abrir una sesión pasiva nueva, conservando la interfaz utilizable en todo momento.
- **FR-066**: La vista de análisis DEBE ofrecer navegación del cursor y selección de rango por teclado, un resumen textual del rango y una tabla accesible equivalente a las series dibujadas.
- **FR-067**: Una sesión pasiva DEBE comenzar al iniciar el muestreo, tras un hueco superior a 60 s y, como máximo, cada 24 h de duración continua. Su informe DEBE congelarse al cerrarla; mientras está activa, el diagnóstico visible es «en vivo» y se marca como provisional. La retención `solo sesión` DEBE borrar los datos al salir de la aplicación.
- **FR-068**: El reloj efectivo DEBE preferir una lectura directa; en su ausencia, el sistema DEBE derivarlo del contador de rendimiento del procesador de Windows (`% Processor Performance` × reloj base) con calidad `derived`, y solo en último término usar el reloj por núcleo del colector con calidad `substitute`. La calidad utilizada DEBE ser visible.
- **FR-069**: La clasificación `térmica confirmada` DEBE exigir una bandera térmica directa. Cuando el equipo no la expone, la ficha de cobertura DEBE indicar «confianza máxima alcanzable: probable» y el motor NO DEBE emitir `confirmada`.
- **FR-070**: El porcentaje de rendimiento disponible PUEDE calcularse con reloj `derived` o `substitute`, pero la confianza resultante NO DEBE superar `media` y el método DEBE indicarse junto al rango.
- **FR-071**: El contexto energético (fuente de alimentación, plan energético activo y porcentaje de batería cuando exista) DEBE registrarse con cada muestra y estar disponible para el motor y para los informes.
- **FR-072**: El usuario DEBE poder marcar una sesión completada como referencia (`baseline` de origen `user_marked`) desde Sesiones o desde el Informe, tras confirmación, y DEBE poder retirar esa marca.
- **FR-073**: Una sesión importada DEBE conservar su informe original y ofrecer una reevaluación con las reglas actuales que se muestra junto al original; su baseline nunca DEBE usarse para la CPU local.
- **FR-074**: Los números y fechas DEBEN formatearse según el idioma efectivo de la interfaz; la temperatura se expresa exclusivamente en grados Celsius en el MVP.
- **FR-075**: Cuando el almacenamiento no esté disponible (disco lleno o base de datos dañada), el muestreo DEBE continuar en memoria, la interfaz DEBE avisar de forma persistente y el sistema DEBE reintentar o recuperar el almacenamiento sin intervención destructiva automática sobre los datos existentes.

### Requisitos no funcionales

- **NFR-001 — Fluidez:** animaciones y navegación a 60 fps en hardware objetivo; los gráficos pueden actualizarse a la frecuencia de muestreo sin bloquear la UI. El material de vidrio (desenfoque de fondo) NO DEBE elevar el consumo pasivo por encima de NFR-003; si lo hace, el nivel baja a `reducido`.
- **NFR-002 — Latencia:** una muestra válida debe verse en menos de 1,5 s en el percentil 95.
- **NFR-003 — Consumo:** monitorización pasiva con objetivo inferior al 1 % de CPU promedio y 180 MB de memoria en un equipo de referencia moderno.
- **NFR-004 — Resiliencia:** la caída del colector no debe cerrar la interfaz; debe reiniciarse con retroceso limitado o pedir intervención.
- **NFR-005 — Accesibilidad:** navegación por teclado, foco visible, contraste WCAG AA, estados no dependientes solo del color y movimiento reducido.
- **NFR-006 — Seguridad:** IPC local con autenticación efímera, esquema validado y lista cerrada de comandos.
- **NFR-007 — Retención:** siete días por defecto; el usuario puede elegir 1, 7, 30 días o solo sesión.
- **NFR-008 — Compatibilidad:** Windows 10 versión 1809 (build 17763) o posterior y Windows 11, de 64 bits, con WebView2 Runtime; las arquitecturas se publican únicamente si todas las dependencias y controladores son compatibles.
- **NFR-009 — Auditabilidad:** el mismo conjunto de muestras y reglas debe producir el mismo diagnóstico.
- **NFR-010 — Inicio:** primera información útil en menos de 5 s en un equipo compatible típico.
- **NFR-011 — Localización:** el 100 % de las claves visibles DEBE existir en ambos catálogos y las variantes inglesa y española no DEBEN romper la ventana mínima ni el escalado de Windows al 200 %.
- **NFR-012 — Ventana:** los controles propios DEBEN ser accesibles por teclado y lector de pantalla y conservar las convenciones esperadas de minimizar, maximizar/restaurar, doble clic y cierre.
- **NFR-013 — Actualizaciones:** toda versión instalable DEBE estar firmada y las operaciones de red NO DEBEN bloquear la UI, el muestreo ni el cierre seguro del colector.
- **NFR-014 — Coherencia de interfaz:** todas las pantallas DEBEN reutilizar el vocabulario visual y los patrones aprobados, sin variantes locales casi equivalentes, y conservar comportamiento y jerarquía en español e inglés, temas claro y oscuro y tamaños compacto, medio y expandido. Los estados aplicables de carga, vacío, datos degradados y error DEBEN estar definidos y ser accesibles.
- **NFR-015 — Apariencia nativa:** ningún botón, fila con estilo de enlace, pestaña o chip DEBE mostrar subrayado al pasar el puntero o recibir foco, ni DEBE usar el cursor de mano; ambos se reservan exclusivamente para la superficie de trazado de `Análisis`. La navegación y las acciones dentro de la aplicación DEBEN implementarse con controles de botón, nunca con hiperenlaces de navegador, salvo para abrir contenido fuera de la aplicación.

### Parámetros iniciales (ruleset v1)

Los valores siguientes son los iniciales de la versión 1 de reglas. Se almacenan en configuración versionada (`ruleset-v1`), son revisables tras validar el corpus de trazas y cualquier cambio exige nueva versión de reglas. Su presencia aquí evita que la implementación los invente.

**Estado térmico**

| Condición | Con TjMax conocido | Sin TjMax |
|---|---|---|
| Normal | margen > 8 °C | < 85 °C |
| Temperatura alta | margen ≤ 8 °C | ≥ 85 °C |
| En el límite / crítica | margen ≤ 3 °C | ≥ 95 °C |

Sin TjMax el anillo del hero indica que la escala es aproximada. Ningún estado térmico implica por sí solo limitación.

**Temperatura representativa**: prioridad `package`/`Tdie` directo → máximo de núcleos → `Tctl` con offset conocido → `Tctl` sin offset (calidad `substitute`). En AMD, si coexisten `Tctl` y `Tdie`, se prefiere `Tdie`. La tarjeta indica cuál se usa.

**Ventanas**: instantánea 1–3 s (solo visualización); corta 30 s (eventos); estable 60–120 s (comparación y cuantificación).

**Evidencia**

| Tipo | Regla v1 |
|---|---|
| Directa persistente | bandera térmica activa en ≥ 50 % de las muestras de una ventana de 30 s con carga ≥ 70 % |
| Correlacionada | margen ≤ 8 °C seguido de caída del reloj efectivo ≥ 8 % respecto a la mediana de los 60 s previos, con carga ≥ 70 % y variación de carga ≤ 15 puntos porcentuales, sostenida ≥ 60 s |
| Eléctrica | bandera de límite de potencia/corriente activa ≥ 50 % de la ventana corta, o potencia estable en el ±3 % de un límite conocido con margen térmico > 8 °C |
| Mixta | evidencia térmica y eléctrica materiales en la misma ventana estable |
| Insuficiente | temperatura sin carga ≥ 70 %, carga variable, o sensores de calidad `substitute` en más de una magnitud crítica |

**Confianza**: puntuación 0..1 calculada a partir de calidad de sensores, cobertura, duración de la evidencia, dispersión y antigüedad del baseline; bandas `baja` < 0,45 ≤ `media` < 0,75 ≤ `alta`. Techos: sin bandera directa → ≤ `media`; reloj `derived`/`substitute` → ≤ `media`; cobertura parcial → ≤ `media`.

**Baseline aprendido**: ventanas de ≥ 120 s con carga ≥ 70 % (variación ≤ 15 pp), margen térmico ≥ 15 °C y sin bandera eléctrica; se requieren ≥ 3 ventanas; caduca a los 30 días o al cambiar la huella de CPU, la versión del normalizador o el contexto energético. Prioridad de uso: `guided` > `user_marked` > `learned`.

**Perfiles de muestreo**

| Perfil | Intervalo | Detalle solicitado | Persistencia por núcleo |
|---|---:|---|---|
| Bajo consumo | 5 s | representativo | no |
| Normal | 1 s | por grupo | no (salvo `sampling.per_core_history`) |
| Diagnóstico | 500 ms | por núcleo | sí |

El detalle por núcleo se mantiene siempre en memoria para la pantalla CPU; el histórico se agrega por grupo salvo en sesiones guiadas o en perfil Diagnóstico.

**Alertas**: tipos `thermal_confirmed`, `power_limited`, `collector_lost` y `guided_finished`; `update_available` pertenece al actualizador. Persistencia mínima 90 s; enfriamiento 30 min por tipo; periodo de silencio opcional (sin valor inicial). Una alerta térmica abre `Análisis` centrado en el evento.

**Prueba guiada**: comprobación ≤ 30 s; reposo opcional 60 s (omitible); calentamiento 90 s; carga sostenida 90 s (`corta`), 180 s (`estándar`) o 300 s (`larga`); recuperación 120 s. Parada automática si el margen es ≤ 1 °C o la temperatura ≥ 100 °C, si faltan 3 muestras consecutivas del sensor crítico o si el generador de carga no responde durante 5 s. Ocultar la ventana o suspender el equipo cancela la prueba. Atajo de parada: `Ctrl+Shift+X`.

**Ventana**: tamaño mínimo 480×600 px lógicos; tamaño inicial 1100×760 centrado en la pantalla principal.

**Atajos**: `Ctrl+1`…`Ctrl+6` navegación en el orden de la barra lateral; `Ctrl+,` Ajustes; `Ctrl+Shift+X` detener prueba; `Ctrl+E` exportar; `F1` ayuda; `Esc` cierra diálogos y tooltips.

**Cierre durante operaciones**: con prueba activa se pregunta «Detener y salir / Cancelar»; con exportación se espera ≤ 5 s y se cancela; con descarga se cancela y descarta; durante instalación no se puede cerrar.

**Exportación**: CSV en formato largo (`timestamp_utc, monotonic_ms, sensor_id, metric, scope, value, status, quality`), separador coma, punto decimal, UTF-8 con BOM, tiempos UTC ISO-8601, nombre `throttlewatch_<tipo>_<fecha>_<id-corto>.<ext>`. La anonimización conserva fabricante, modelo comercial, topología y versiones; elimina nombre de equipo, usuario, números de serie, direcciones MAC, rutas, huella de monitores y GUID de plan energético.

**Almacenamiento**: datos en `%LOCALAPPDATA%\ThrottleWatch`; registros en `logs/` con rotación de 5 ficheros × 5 MB y nivel `info` (nivel `debug` como ajuste avanzado). Disco lleno: modo solo memoria con aviso y reintento cada 60 s. Base de datos dañada: se renombra a `.corrupt-<fecha>`, se crea una nueva y se ofrece exportar la dañada.

### Entidades clave

- **Dispositivo CPU**: identidad normalizada, fabricante, familia, topología y capacidades.
- **Descriptor de sensor**: sensor original, magnitud normalizada, unidad, alcance y calidad.
- **Muestra**: valores coincidentes en un instante monotónico y su estado de validez.
- **Sesión**: intervalo de monitorización o prueba, contexto energético y resultado.
- **Contexto energético**: fuente de alimentación, plan energético activo y batería, registrados con cada muestra.
- **Baseline**: referencia local por carga, topología, perfil energético y versión.
- **Evento de limitación**: tipo, inicio, fin, severidad y evidencias.
- **Diagnóstico**: clasificación, confianza, estimación opcional y explicación.
- **Preferencias**: onboarding, muestreo, retención, alertas, privacidad, idioma, apariencia, ciclo de vida, ventana, inicio con Windows y actualizaciones.
- **Estado de actualización**: habilitación, última comprobación automática, versión disponible, descarga verificada, progreso y último error.

## Criterios de éxito medibles

- **SC-001**: En pruebas moderadas con personas (no automatizables en CI), al menos el 90 % de nuevos usuarios identifica correctamente el estado principal en menos de 10 s.
- **SC-002**: El 100 % de los diagnósticos cuantificados muestra baseline, intervalo comparado, rango y confianza.
- **SC-003**: Ninguna traza “caliente sin caída demostrada” del conjunto de regresión se etiqueta como limitación térmica confirmada.
- **SC-004**: Al menos el 95 % de las trazas etiquetadas con señal directa persistente se clasifica como limitación térmica confirmada.
- **SC-005**: El motor distingue correctamente térmica, potencia, mixta e indeterminada en al menos el 90 % del corpus validado.
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
- La disponibilidad de PawnIO o equivalente se trata como una capacidad detectable, no como certeza.
- El usuario puede aceptar elevación durante instalación/reparación, pero la interfaz diaria no la necesita.
- La referencia local es más honesta que una base de datos universal de puntuaciones.
- El nombre definitivo es `ThrottleWatch`; el icono, identidad visual final y modelo de distribución se cerrarán antes del empaquetado público.
- Activar el actualizador constituye consentimiento para su aviso nativo de versión disponible; el interruptor general de notificaciones controla alertas térmicas, no ese aviso solicitado por separado.
- Un perfil de datos por usuario de Windows; no hay sincronización entre usuarios ni equipos.
- Una única CPU física (un socket); se toma la primera si el sistema expone varias.
- La temperatura se expresa solo en grados Celsius.
- ThrottleWatch no se ejecuta como servicio de Windows; en bandeja sigue siendo un proceso del usuario.
- El sidecar se ejecuta con los mismos privilegios que la interfaz (sin elevación) salvo que el spike de acceso de bajo nivel demuestre otra necesidad y se documente como excepción.
- Las máquinas virtuales no son un caso principal: se soportan solo con cobertura reducida y estado explícito.
- `LibreHardwareMonitorLib` no expone la clase de núcleo (P/E/LP); el sidecar la deriva de `GetLogicalProcessorInformationEx` y CPUID (hoja 0x1A en Intel). Cuando no puede, los grupos se marcan `unknown`.
- Es probable que `LibreHardwareMonitorLib` no exponga reloj efectivo ni banderas térmicas/eléctricas directas; el diseño asume que el reloj efectivo se deriva del contador de rendimiento de Windows y que `térmica confirmada` solo existe cuando una bandera directa está disponible (lectura MSR por acceso de bajo nivel o sensor equivalente).
- El idioma del sistema se toma del primer idioma de visualización de Windows; el tema del sistema, del modo de aplicación (`AppsUseLightTheme`).
- La documentación esencial se empaqueta con la aplicación; «Documentación en línea» abre el navegador del sistema por gesto explícito del usuario y es la única otra acción con red además del actualizador.
- El instalador es por usuario y sin elevación; el acceso de bajo nivel, si el spike lo aprueba, se instala en un paso separado por máquina. Desinstalar pregunta si conservar los datos.

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
