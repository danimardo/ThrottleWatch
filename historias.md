# Historias de usuario de ThrottleWatch

## Propósito

Este documento reconstruye las necesidades de usuario que dan origen a las especificaciones de ThrottleWatch. Actúa como fuente narrativa previa al lenguaje normativo: explica quién necesita cada capacidad, qué problema intenta resolver, qué resultado espera y qué límites deben conservarse al diseñarla.

El público principal es una persona usuaria de Windows sin conocimientos especializados de hardware. El detalle técnico existe, pero aparece progresivamente y con explicaciones breves.

## Principios comunes

- ThrottleWatch observa y diagnostica; no modifica voltajes, potencia, frecuencias, BIOS ni ventiladores.
- Una temperatura alta no demuestra por sí sola pérdida de rendimiento.
- Una conclusión distingue observación, inferencia y confirmación.
- La ausencia de datos se presenta como desconocida, nunca como cero.
- Todo funciona localmente y sin Internet; la única red opcional es el actualizador activado expresamente. No hay envío de informes de fallo ni de uso, ni siquiera anónimos.
- El usuario controla permisos, segundo plano, datos, pruebas y actualizaciones.
- Toda la experiencia está disponible en español e inglés, en claro y oscuro y mediante teclado.
- Los números que la aplicación usa para decidir (umbrales, ventanas, duraciones) están escritos y versionados; no se dejan al criterio de quien programa.
- El producto cubre un equipo con una CPU, un usuario de Windows y temperaturas en grados Celsius.

## HU-01 — Entender el estado actual de la CPU (P1)

**Necesidad:** una persona nota calor, ruido o lentitud y quiere una respuesta rápida sin interpretar decenas de sensores.

**Historia:** Como usuario corriente, quiero abrir ThrottleWatch y saber en pocos segundos si mi CPU funciona con normalidad, para decidir si necesito investigar algo más.

**Criterios de aceptación:**

- `Ahora` muestra una conclusión principal, CPU, temperatura, carga, frecuencia activa (la velocidad real mientras trabaja) frente a la garantizada, potencia y frescura.
- Una franja fija sobre la conclusión indica el procesador y su topología, si el equipo va a corriente o a batería, si el colector está conectado y hace cuánto llegó la última muestra; con datos de más de cinco segundos avisa de que están obsoletos.
- El estado usa texto, icono y color; un sensor ausente dice `No disponible`.
- Las CPU híbridas distinguen grupos P, E y LP.
- `Ver cobertura` abre una tabla que dice, magnitud por magnitud, qué se mide, con qué calidad y de qué sensor, qué falta y por qué, y cuál es la conclusión más fuerte que este equipo permite alcanzar («confianza máxima: probable»).
- Si el equipo no expone el reloj real, la aplicación lo estima con los contadores de Windows y lo dice; si tampoco es posible, usa el reloj nominal y lo marca como sustituto.
- Cuando el colector está desconectado la conclusión pasa a «Datos insuficientes» y la franja explica por qué.

**Casos límite:** colector desconectado, sensor obsoleto, CPU desconocida, cobertura parcial, máquina virtual sin sensores y reanudación tras suspensión conservan una interfaz útil.

## HU-02 — Saber qué limita el rendimiento (P1)

**Necesidad:** una frecuencia baja puede deberse a calor, potencia, batería, perfil energético, al propio fabricante del portátil o simplemente a que terminó el turbo inicial; una etiqueta simplista puede provocar una intervención innecesaria o, al revés, disuadir de una que sí ayudaría.

**Historia:** Como usuario, quiero que ThrottleWatch distinga limitación térmica, de potencia, del propio equipo, mixta y falta de evidencia, y que me diga si lo que ocurre es normal o un problema, para actuar sobre la causa correcta.

**Criterios de aceptación:**

- «Confirmada» exige que el propio procesador declare que está limitando por temperatura; una señal de «PROCHOT» sin ese motivo no cuenta como calor, porque puede venir del cargador, la batería o la placa.
- Sin esas señales, la aplicación mira **qué se queda clavado en su tope**: si es la temperatura, el límite es térmico; si es la potencia con la CPU fresca, es de potencia. Eso se comunica como «compatible», nunca como «confirmado».
- La bajada de velocidad que ocurre cuando termina el turbo de los primeros segundos o minutos es normal y se muestra como tal, no como un problema térmico.
- Si el fabricante del portátil va bajando la potencia porque el equipo se calienta por fuera, la aplicación lo dice y **sí** recomienda mejorar la ventilación, aunque la CPU no esté al límite.
- Llegar al límite de temperatura no es un problema si el procesador sigue por encima de la frecuencia que el fabricante garantiza: se muestra como «dentro de especificación». Solo por debajo de esa frecuencia se trata como problema y puede generar un aviso.
- Una limitación de potencia con margen térmico no recomienda primero mejorar la refrigeración.
- En un juego que usa pocos núcleos, la aplicación evalúa esos núcleos y no descarta el análisis porque la carga total sea baja.
- Cada diagnóstico muestra evidencias, ausencias, alternativas, intervalo y confianza; si no puede atribuir causa, explica qué información falta.
- La aplicación dice desde el principio qué nivel de detalle alcanza en este equipo (completo, con potencia o básico) y qué gana instalando el acceso avanzado.
- Con el equipo a batería, con un plan de energía restrictivo o con modos de eficiencia de Windows, el diagnóstico lo tiene en cuenta como causa alternativa.

## HU-03 — Saber si enfriar mejor merece la pena (P1)

**Necesidad:** el usuario quiere saber si una base refrigerada, una limpieza o una pasta térmica nueva le devolverían rendimiento, pero una cifra mal calculada puede engañarle en cualquiera de los dos sentidos.

**Historia:** Como usuario, quiero una estimación prudente de cuánto ganaría con mejor refrigeración y, si hago una prueba, el rendimiento que mi equipo sostiene medido de verdad, para decidir si merece la pena actuar.

**Criterios de aceptación:**

- La estimación se basa en cuánta potencia le queda al procesador por usar cuando el calor lo frena; solo existe cuando el equipo permite leer su límite de potencia.
- Se presenta como un tramo claro («apenas mejoraría», «mejora moderada», «mejora notable») y un rango redondeado de 5 en 5, nunca con decimales.
- Si el límite es de potencia, la aplicación dice que la refrigeración apenas influye.
- Sin los datos necesarios no hay cifra: se explica qué falta.
- En el diagnóstico guiado, la aplicación **mide** el trabajo que hace el procesador con su propia carga de prueba y muestra cuánto rinde al final frente al principio, y a qué se debe la diferencia (fin del turbo, potencia, temperatura o el equipo).
- Las comparaciones «antes/después» se hacen entre dos diagnósticos guiados iguales (misma duración, misma alimentación), y solo esos pueden marcarse como referencia.
- El tiempo en throttling nunca equivale a porcentaje de rendimiento perdido, y el turbo anunciado en la caja nunca se usa como referencia.
- Las CPU híbridas se calculan con los núcleos que están trabajando, por grupos.

## HU-04 — Ejecutar un diagnóstico guiado seguro (P2)

**Historia:** Como usuario, quiero una prueba guiada, voluntaria y cancelable, para comparar el comportamiento frío y caliente sin poner en riesgo mi equipo.

**Criterios de aceptación:**

- Antes de empezar se explica, en una lista, qué carga se aplicará, cuánto durará, qué sensores se usarán, cuándo se detendrá sola y que no es un benchmark.
- Las fases son: comprobación, reposo opcional (se puede omitir), calentamiento, carga sostenida, recuperación y resultado; cada una muestra tiempo restante, temperatura, límite, margen, velocidad frente a la garantizada y el trabajo medido en vivo.
- La duración se elige en Ajustes entre corta (3 min de carga), estándar (4 min) y larga (6 min); siempre es lo bastante larga para medir después de que termine el turbo inicial.
- `Detener ahora` está siempre visible, tiene atajo de teclado y también aparece en una barra fija en cualquier otra pantalla mientras la prueba dure; `Esc` no la detiene.
- Cancelar cesa la carga y conserva datos parciales como incompletos.
- Llegar al límite de temperatura no detiene la prueba: el procesador se protege solo y eso es justo lo que se quiere observar. La prueba se detiene sola si la temperatura supera ese límite (el procesador no se está protegiendo), si la velocidad se hunde por debajo de la mitad de la garantizada estando al límite (refrigeración gravemente insuficiente), si se pierde el sensor crítico o si el generador de carga deja de responder; los límites se ven en Ajustes pero no se pueden cambiar.
- Ocultar la ventana o suspender el equipo cancela la prueba y el resultado lo explica.
- En batería advierte de posibles límites de potencia; si el usuario lo prefiere, puede exigir corriente para empezar. Al terminar, puede avisar con una notificación si están activadas.
- Al terminar ofrece usar el resultado como referencia.
- Nunca empieza desde el onboarding sin un gesto específico.

## HU-05 — Explorar la evolución y la causa (P2)

**Historia:** Como usuario que desea más detalle, quiero gráficos sincronizados y eventos, para entender la secuencia que llevó al diagnóstico.

**Criterios de aceptación:**

- Temperatura, reloj, carga y potencia comparten cursor y eje temporal.
- Eventos térmicos, eléctricos y mixtos muestran tipo, duración y evidencia.
- Los huecos se dibujan como ausencia, no como cero ni interpolación.
- El mapa de núcleos distingue grupos y permite alternar magnitud.
- Los gráficos tienen alternativa textual, teclado y tabla del rango.

## HU-06 — Vigilar en segundo plano sin molestias (P2)

**Historia:** Como usuario, quiero permitir que ThrottleWatch continúe en la bandeja y me avise solo de episodios relevantes, para detectar problemas durante el uso real.

**Criterios de aceptación:**

- La bandeja solo continúa si se habilita expresamente; no existe «en la bandeja sin medir».
- Las notificaciones están apagadas de fábrica; hay un botón para probar cómo se ven.
- Los avisos posibles son cuatro: limitación térmica confirmada, límite de potencia, colector perdido y prueba guiada terminada. Exigen que la situación dure al menos minuto y medio, no se repiten antes de media hora y respetan un periodo de silencio opcional.
- Pulsar un aviso térmico abre `Análisis` centrado en ese momento.
- El icono diferencia normal, aviso, crítico, desconocido y desconectado; un clic muestra u oculta la ventana y el menú ofrece estado, abrir, pausar o reanudar y salir.
- En batería el usuario elige entre mantener el muestreo, bajarlo a bajo consumo o pausarlo; de fábrica se mantiene.

## HU-07 — Exportar e importar evidencia privada (P3)

**Historia:** Como usuario, quiero exportar e importar sesiones con control de privacidad, para comparar resultados o solicitar ayuda.

**Criterios de aceptación:**

- Se exportan muestras CSV e informe JSON versionado, desde una sesión, desde un informe o desde un tramo seleccionado en `Análisis`, siempre con el mismo diálogo.
- Antes de exportar se muestran campos incluidos y excluidos, el tamaño estimado y el nombre propuesto; la carpeta la elige el diálogo normal de Windows.
- La anonimización elimina identificadores mediante allowlist: se conservan fabricante, modelo comercial, topología y versiones; se eliminan nombre del equipo, usuario, números de serie, direcciones de red, rutas y el identificador del plan de energía.
- Importar se hace desde `Sesiones`; el resultado explica si el fichero se validó, se migró o tiene avisos. Una sesión importada conserva su informe original y puede reevaluarse con las reglas actuales viendo ambas versiones; su referencia nunca se aplica a este equipo.
- Una sesión compatible se reproduce sin sensores reales.
- CSV/JSON usan claves técnicas inglesas, separador coma, punto decimal y tiempos en UTC; el informe humano usa el idioma efectivo.

## HU-08 — Comprender la aplicación en el primer inicio (P1)

**Historia:** Como nuevo usuario, quiero una introducción breve y omitible, para empezar con expectativas correctas y sin permisos inesperados.

**Criterios de aceptación:**

- Hay cinco diapositivas: bienvenida, señales, conclusiones, privacidad/preferencias y detección.
- *Thermal throttling*, el límite de temperatura (TjMax) y la frecuencia garantizada (frecuencia base) incluyen explicaciones sencillas.
- Puede omitirse sin confirmación desde las cuatro primeras diapositivas, reanudarse y repetirse desde Ayuda.
- La quinta diapositiva inicia detección pasiva sin UAC.
- La cobertura parcial permite continuar. El acceso avanzado tiene cinco situaciones claras: no hace falta, disponible, instalable, bloqueado por el sistema o error; solo «instalable» ofrece instalar o reparar, y siempre con un gesto explícito.
- `Abrir Ahora` es principal y el diagnóstico guiado secundario.
- Una actualización importante muestra solo novedades pertinentes como tarjetas breves sobre `Ahora`, con «Entendido».

## HU-09 — Usar el idioma adecuado (P1)

**Historia:** Como usuario, quiero que ThrottleWatch elija un idioma útil desde Windows y me permita anularlo, para entender toda la interfaz desde el primer momento.

**Criterios de aceptación:**

- `Usar idioma del sistema` es inicial.
- `es`, `ca`, `gl`, `eu`, `ast` y `an` usan español, incluido `ca-AD`.
- Cualquier otro locale, incluido `pt`, usa inglés.
- `Español` y `English` prevalecen sobre Windows.
- En modo sistema, un cambio de idioma se recoge en el siguiente inicio; se toma el primer idioma de visualización de Windows, no la región.
- Ambas traducciones cubren onboarding, accesibilidad, bandeja, errores y notificaciones.
- Números y fechas siguen el idioma de la interfaz (`98,5 °C` en español, `98.5 °C` en inglés); la temperatura se muestra siempre en grados Celsius.

## HU-10 — Adaptar apariencia y movimiento (P1)

**Historia:** Como usuario, quiero elegir tema y movimiento o seguir Windows, para leer cómodamente ThrottleWatch.

**Criterios de aceptación:**

- Existen `Sistema`, `Claro` y `Oscuro`; `Sistema` es inicial y reacciona en caliente.
- Ambos temas cumplen contraste y conservan la semántica.
- El movimiento se elige entre `Sistema`, `Reducido` y `Completo`; sigue Windows inicialmente.
- La interfaz tiene un acabado de vidrio (superficies translúcidas que dejan intuir el fondo, con borde luminoso), pero nunca tan transparente que cueste leer; se puede rebajar o quitar (`Sistema` sigue los «Efectos de transparencia» de Windows) y la aplicación lo rebaja sola si el equipo va justo.
- La aplicación se comporta como un programa de Windows, no como una página web abierta dentro de la ventana: nada se subraya al pasar el ratón por encima ni al recibir el foco, y los botones no muestran el cursor de mano; ese cursor se reserva para los puntos interactivos del gráfico de `Análisis`.
- Las animaciones acompañan las transiciones y los cambios de estado (entrar en una pantalla, cambiar la conclusión, abrir un menú o un diálogo, pulsar un botón); en reposo la interfaz está quieta. Con movimiento reducido, todo es instantáneo.
- La interfaz funciona al 200 % de escala sin ocultar acciones críticas.

## HU-11 — Tener una ventana integrada y predecible (P1)

**Historia:** Como usuario, quiero una barra superior propia y que la ventana recuerde dónde la dejé, para que se comporte como una aplicación de escritorio cuidada.

**Criterios de aceptación:**

- La barra muestra icono, nombre, arrastre, minimizar, maximizar/restaurar y cerrar.
- Doble clic en el área de arrastre maximiza o restaura.
- Los controles son accesibles y aparecen también en onboarding y errores.
- Se recuerdan posición, tamaño, maximizado y último rectángulo restaurado, no minimizado. La ventana nunca baja de 480×600 y la primera vez abre a 1100×760 centrada.
- Si el monitor desaparece, la ventana reaparece dentro de la pantalla principal.
- Abrir ThrottleWatch cuando ya está abierto solo trae la ventana existente al frente.
- Cerrar con una prueba en curso pregunta «Detener y salir»; con una exportación espera unos segundos; con una descarga la cancela; durante una instalación no se puede cerrar.

## HU-12 — Elegir cierre e inicio con Windows (P2)

**Historia:** Como usuario, quiero decidir una vez qué hace la X y poder cambiarlo después, para controlar la ejecución en segundo plano.

**Criterios de aceptación:**

- La primera X pregunta entre «Salir» y «Continuar en la bandeja y seguir midiendo», guarda la elección e indica Ajustes; elegir bandeja activa la monitorización en segundo plano. Cerrar la pregunta con `Esc` no guarda nada y la ventana sigue abierta.
- Cierres posteriores la aplican. En Ajustes la elección aparece como dos opciones y, mientras no se haya decidido, ninguna está marcada y se lee «Se te preguntará al cerrar».
- `Iniciar con Windows` está apagado inicialmente.
- Al activarlo permite mostrar ventana o iniciar oculto; oculto exige bandeja habilitada.
- Deshabilitar bandeja corrige una configuración incompatible (inicio oculto y cierre a bandeja) en el mismo gesto y lo dice.

## HU-13 — Configurar sin conocimientos técnicos (P2)

**Historia:** Como usuario corriente, quiero ajustes agrupados y valores iniciales prudentes, para adaptar ThrottleWatch con confianza.

**Criterios de aceptación:**

- Ajustes agrupa General, Idioma, Apariencia, Monitorización, Bandeja y notificaciones, Datos y privacidad, Sensores y cobertura, Diagnóstico, Actualizaciones, Acerca de y ayuda, y Zona de riesgo.
- Los perfiles son `Bajo consumo` (cada 5 s), `Normal` (cada segundo) y `Diagnóstico` (dos veces por segundo, con detalle por núcleo); los números quedan en avanzado.
- La retención ofrece solo sesión, 1, 7 y 30 días; siete días es inicial. Se ve el espacio que ocupan la base de datos y los registros.
- Cada sección tiene, si hace falta, un bloque «Avanzado» plegado; nada avanzado aparece desplegado la primera vez.
- Los controles dependientes permanecen visibles y explican por qué se deshabilitan.
- Acerca de ofrece la ayuda incluida, un enlace a la documentación en línea (abre el navegador y se indica), las licencias de terceros, un resumen técnico copiable sin identificadores y la carpeta de registros.
- No existe ninguna opción que envíe datos fuera del equipo.

## HU-14 — Borrar datos o restablecer por completo (P2)

**Historia:** Como usuario, quiero distinguir borrar diagnósticos de restablecer ThrottleWatch, para elegir exactamente el alcance de una acción destructiva.

**Criterios de aceptación:**

- `Eliminar todos mis datos` borra sesiones, muestras, eventos, informes y referencias tras confirmar.
- Conserva idioma, tema, ajustes, onboarding y geometría.
- `Restablecer ThrottleWatch` tiene advertencia distinta, borra datos y preferencias, desregistra el inicio y elimina descargas pendientes.
- El siguiente arranque tras restablecer equivale a una instalación nueva.
- Ambas acciones son transaccionales y comunican fallos parciales.
- Desinstalar ThrottleWatch pregunta si conservar los datos.

## HU-15 — Actualizar voluntariamente y con firma (P3)

**Historia:** Como usuario, quiero activar la búsqueda de versiones y decidir por separado si descargo e instalo, para actualizar con control y garantías.

**Criterios de aceptación:**

- El actualizador está apagado de fábrica y apagado significa cero tráfico.
- Activado, comprueba automáticamente como máximo cada 24 horas (al arrancar si toca y mientras está abierta).
- `Buscar actualizaciones` permite reintentar manualmente antes.
- Solo hay versiones estables; no existe canal beta.
- Una versión disponible avisa en Ajustes y Windows, pero no descarga.
- `Descargar` es otro gesto, muestra progreso y verifica la firma; solo cuando está verificada aparece `Instalar`.
- `Instalar` requiere otro gesto, anuncia el cierre y rechaza parciales o firmas inválidas.
- La instalación se bloquea, con el motivo a la vista, durante un diagnóstico, una exportación, una importación o un borrado.
- Los fallos no afectan al resto de la aplicación.
- Se usa un endpoint fijo de GitHub Releases sin identificadores ni telemetría.

## HU-16 — Comprender y conservar la privacidad (P1 transversal)

**Historia:** Como usuario, quiero saber qué se guarda y cuándo existe conexión de red, para confiar en ThrottleWatch.

**Criterios de aceptación:**

- Muestreo, diagnóstico, historial e informes funcionan sin Internet.
- No existe telemetría remota ni envío de informes de fallo, ni siquiera anónimos u opcionales.
- Retención y espacio utilizado son visibles.
- La exportación anónima describe lo que excluye.
- La única red prevista en el MVP es el actualizador activado expresamente; abrir la documentación en línea usa el navegador del sistema y solo ocurre si el usuario lo pide.
- El resumen técnico que se puede copiar para pedir ayuda no contiene identificadores del equipo ni del usuario.

## HU-17 — Usar una interfaz coherente y predecible (P1 transversal)

**Historia:** Como usuario, quiero que todas las pantallas se comporten y se presenten de forma coherente, para aprender ThrottleWatch una sola vez y poder utilizarlo con independencia de mi idioma, tema, tamaño de ventana o forma de interacción.

**Criterios de aceptación:**

- Las mismas acciones, estados y conceptos usan componentes, vocabulario y jerarquías visuales consistentes en toda la aplicación.
- Todas las pantallas funcionan en español e inglés, en claro y oscuro y en los tamaños compacto, medio y expandido previstos.
- Cada flujo presenta de forma coherente los estados de carga, vacío, datos degradados y error que le correspondan.
- Las acciones esenciales son accesibles mediante teclado, muestran foco visible y no comunican el estado solo mediante color.
- Los gráficos ofrecen navegación por teclado y una alternativa textual o tabular equivalente.
- La barra superior y los controles de escritorio propios mantienen el mismo acabado visual que el resto de la aplicación.
- En ventana estrecha la navegación inferior mantiene `Ahora`, `Análisis`, `CPU` y un `Más` desde el que se llega a `Sesiones`, `Diagnóstico guiado` y `Ajustes`; nada deja de ser alcanzable.
- Los avisos que afectan a toda la aplicación (prueba en curso, colector con problemas) aparecen en una barra fija bajo el título, en cualquier pantalla.
- Atajos: `Ctrl+1`…`Ctrl+6` para navegar, `Ctrl+,` Ajustes, `Ctrl+Shift+X` detener prueba, `Ctrl+E` exportar, `F1` ayuda.

## HU-18 — Repasar y organizar sesiones (P2)

**Necesidad:** el diagnóstico de un momento concreto solo tiene sentido si puede recuperarse después y compararse con otros.

**Historia:** Como usuario, quiero ver la lista de lo que ThrottleWatch ha observado, abrir cualquier informe, decidir cuál es mi referencia y eliminar lo que no me interesa, para tener un historial manejable.

**Criterios de aceptación:**

- `Sesiones` lista sesiones pasivas, guiadas e importadas, con estado (en curso, completada, cancelada, incompleta con motivo, importada), fecha, duración, resultado y marca «Referencia».
- Una sesión pasiva empieza al arrancar la medición y se parte automáticamente tras una pausa larga (suspensión, reinicio del colector) o al cumplir 24 horas, para que cada informe describa un tramo homogéneo.
- Mientras una sesión está en curso su informe se ve como «provisional»; al cerrarse se congela con las reglas y la referencia que se usaron.
- Cada sesión puede abrirse, exportarse y eliminarse (con confirmación); la sesión en curso no se elimina.
- Con retención «solo esta sesión», todo se borra al salir de la aplicación.

## HU-19 — Seguir siendo útil cuando algo falla (P1 transversal)

**Necesidad:** el equipo real tiene discos llenos, colectores que se cierran y bases de datos que se corrompen; la aplicación no puede convertir eso en una pantalla en blanco.

**Historia:** Como usuario, quiero que ThrottleWatch me diga qué ha fallado, siga mostrando lo que pueda y me ofrezca la salida, para no perder la confianza en lo que veo.

**Criterios de aceptación:**

- Si el colector se cae, la interfaz se conserva, los datos se marcan obsoletos, se reintenta un número limitado de veces y después se pide intervención con un resumen técnico.
- Si el disco está lleno, se sigue midiendo en memoria, se avisa de forma persistente y se reintenta guardar periódicamente.
- Si la base de datos está dañada, se aparta con fecha, se crea una nueva y se ofrece exportar la dañada; nunca se borra sin avisar.
- Un fallo del actualizador, de una exportación o de una importación se contiene en su sección y no afecta al estado térmico principal.
- Los avisos de estos fallos aparecen en una barra fija visible desde cualquier pantalla, con las acciones de reintentar y ver el resumen técnico.

## Mapa hacia las especificaciones

- HU-01 a HU-07 originan las historias funcionales principales de `spec.md`.
- HU-08 a HU-11 originan onboarding, localización, apariencia y ventana.
- HU-12 a HU-14 originan Ajustes, ciclo de vida y gestión de datos.
- HU-15 origina el actualizador voluntario y firmado.
- HU-16 es transversal a constitución, especificación y contratos.
- HU-17 origina los requisitos transversales de coherencia visual, accesibilidad y adaptación y se materializa mediante el sistema de diseño aprobado.
- HU-18 origina la pantalla `Sesiones`, los límites de sesión pasiva y la referencia marcada por el usuario.
- HU-19 origina los requisitos de resiliencia (colector, disco, base de datos) y la barra de avisos global.
- Los valores concretos que estas historias mencionan (duraciones, intervalos, umbrales) están fijados en `spec.md` como parámetros iniciales versionados; las historias los citan para que el cliente reconozca lo que pidió, no para congelarlos.

Las decisiones técnicas —Tauri, Svelte, Rust, .NET, SQLite, LibreHardwareMonitor y Ed25519— pertenecen al plan y a la investigación. Estas historias expresan necesidades y resultados observables; no obligan por sí solas a una implementación concreta.
