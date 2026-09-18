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

## Estrategia integral de testing para SvelteKit

> **Adaptación obligatoria al proyecto real.** ThrottleWatch **no usa SvelteKit**: es una aplicación de escritorio Windows con interfaz Svelte + Vite (SPA renderizada en cliente) dentro de una WebView2 gestionada por Tauri, un backend Rust en el mismo proceso y un colector .NET como proceso auxiliar. La constitución (`.specify/memory/constitution.md`) prohíbe SvelteKit. Por eso esta estrategia conserva la exigencia de calidad del enfoque SvelteKit, pero traduce cada técnica a su equivalente real o la declara «no aplica» con su motivo (§ 4).
>
> **Jerarquía.** Esta sección es una guía operativa para los agentes que planifican, implementan y verifican. No sustituye a la constitución, `spec.md` ni `plan.md`; si hay conflicto, prevalece la constitución (en especial los principios VII, IX, XII, XIII, XIV, XVI y XVII y las puertas de calidad). Las versiones de herramientas **no se repiten aquí**: la fuente es la tabla «Versiones fijadas» de la constitución y los archivos de bloqueo del repositorio.

### 0. Principios de esta estrategia

1. **Diseñar las pruebas antes de olvidar el comportamiento:** cada lote empieza con su plan de pruebas (§ 8), aunque los tests se escriban al cierre.
2. **Nivel más barato capaz de demostrar el requisito:** un comportamiento se prueba donde vive su lógica. La lógica de negocio vive en Rust (principio IX), así que la mayoría de criterios de aceptación se demuestran con pruebas Rust sin interfaz ni hardware.
3. **Lotes funcionales, no función a función:** se implementa un bloque coherente, se estabiliza su contrato y se completan sus tests antes de pasar al siguiente (§ 7).
4. **Feedback escalonado:** durante la edición se ejecuta lo más cercano; las suites caras (E2E de aplicación, matriz visual, mutation testing, rendimiento) quedan para checkpoints superiores (§ 27).
5. **Sin hardware en CI:** todo lo automatizable se ejecuta con trazas, fakes y el colector de reproducción (*replay*). El hardware real se reserva para el corpus, la matriz de hardware y las verificaciones de release.
6. **La cobertura es una señal, no el objetivo:** los umbrales son constitucionales, pero la calidad se juzga con criterios de aceptación trazados, corpus etiquetado y mutation testing selectivo.

### 1. Clasificación del proyecto

| Pregunta | Respuesta para ThrottleWatch |
|---|---|
| Modelo | **Combinación:** aplicación de escritorio con interfaz SPA renderizada en cliente (CSR) + backend nativo Rust expuesto por comandos y eventos Tauri + proceso auxiliar .NET por `stdin/stdout`. |
| ¿Sitio SSR, prerenderizado o full-stack SvelteKit? | No. No hay servidor HTTP, SSR, hidratación ni prerendering. |
| ¿Consume una API? | Sí, pero local: comandos y eventos Tauri (`contracts/application-commands.md`) y protocolo NDJSON con el colector (`contracts/ipc-protocol.md`, `telemetry.schema.json`). |
| Usuarios y autenticación | Ninguno: un único usuario de Windows y sin cuentas. Los riesgos equivalentes son de **integridad y autorización de canal** (§ 14.6). |
| Persistencia | Una base SQLite local gestionada solo por Rust (principio X). |
| Red | Cero, salvo el actualizador activado expresamente (principio V). |
| Estado del repositorio (2026-09-18) | Especificación, plan, tareas, contratos y sistema de diseño completos. **No existe todavía código de aplicación, pruebas, configuración de Vitest o Playwright ni CI.** Solo `design/harness/` y `design/mockup/` tienen `check` y `build`. |

### 2. Inventario de arquitectura y runtimes

| Capa | Ubicación prevista (`plan.md`) | Runtime real | Runtime de las pruebas rápidas | ¿Necesita navegador real? |
|---|---|---|---|---|
| Interfaz | `apps/desktop/src/` (features, stores, puente, `design-system/` copiado) | WebView2 Evergreen (Chromium de Edge) | Node + jsdom (Vitest) | Solo para maquetación, CSS calculado, vidrio (`backdrop-filter`), foco visible, teclado real, árbol de accesibilidad y capturas |
| Backend | `apps/desktop/src-tauri/src/` (`diagnostics`, `ipc`, `storage`, `export`, `telemetry`, comandos) | Proceso nativo Tauri | `cargo nextest` nativo, sin ventana | No |
| Colector | `apps/sensor-agent/` | .NET autocontenido `win-x64` | xUnit | No; LibreHardwareMonitorLib real solo en Windows |
| Contratos | `packages/contracts/` (esquemas + fixtures) | Las tres capas | Rust, TypeScript (Zod + ajv) y C# | No |
| Trazas | `packages/trace-fixtures/` (sintéticas, grabadas y etiquetadas) + colector falso/replay (T016) | Pruebas y modo replay | Todas | No |
| Sistema de diseño | `design/` (fuente) → copia sincronizada | Harness Vite | `check` + `build` del harness | Para verificación visual del harness |

Superficies que Playwright **no puede** alcanzar porque son nativas: icono y menú de bandeja, notificaciones de Windows, diálogos nativos de archivos, UAC y controlador, registro de inicio con Windows, instalador, varios monitores y la decoración de ventana del sistema. Se prueban mediante **puertos sustituibles en Rust** (§ 14) y una **lista de verificación manual de release** con evidencias (§ 31.4).

### 3. Código de backend, interfaz y compartido

Equivale a la distinción servidor/navegador/compartido de SvelteKit:

| Tipo | Qué contiene | Dónde se prueba |
|---|---|---|
| **Backend (servidor)** | Motor de diagnóstico puro, normalización de dominio, sesiones, alertas, potencial, agregación temporal, persistencia, exportación e importación, anonimización, actualizador, ciclo de vida, validación de comandos y del protocolo | Rust: unitarias, integración y aceptación por corpus |
| **Colector** | Adquisición LibreHardwareMonitorLib, clasificación P/E/LP, lectura MSR/SMU, normalización de sensores, protocolo | C#: unitarias con hardware falso; integración real solo en Windows |
| **Interfaz (navegador)** | Presentación, navegación, formularios de ajustes, estado de presentación, adaptadores vista→props, puente validado con Zod, catálogos, formato `Intl`, envoltorio de registros | Vitest (unitarias y componentes en jsdom) + Playwright |
| **Compartido** | Esquemas JSON, fixtures de contrato, trazas, catálogo de códigos de error y de evento | Las tres capas contra el mismo corpus |

Requiere **ambos lados** (interfaz real + backend real): arranque sin destellos, cambio de idioma o tema sin perder contexto, primera X, bloqueo de cierre, reproducción de sesiones, exportación de rango desde `Análisis` y banners globales ante fallos del colector o del disco.

### 4. Equivalencias con las técnicas pedidas para SvelteKit

| Técnica | ¿Aplica? | Equivalente en ThrottleWatch |
|---|---|---|
| Rutas, `+page`, `+layout` | Parcial | Enrutador interno del shell (Ahora, Análisis, CPU, Sesiones, Diagnóstico guiado, Ajustes, Informe) y puerta de onboarding. La decisión de destino es una función pura con prueba unitaria; la navegación se prueba con componentes y E2E. |
| `load` / server `load` | Parcial | Primer render con `get_live_snapshot` y suscripción posterior a eventos (`telemetry:snapshot`, `collector:state`…). Se prueba el adaptador del puente con respuestas simuladas. |
| Server Actions y formularios | Sí, equivalente | Ajustes, diálogos y confirmaciones que invocan comandos (`set_preference`, `delete_monitoring_data`, `start_guided`…). Zod valida en la interfaz y Rust revalida (principio XIV). |
| Endpoints `+server` y APIs | Sí, equivalente | Comandos y eventos Tauri y protocolo NDJSON con el colector. |
| Hooks | Parcial | `setup` de Tauri, plugins (instancia única, autostart, notificaciones, actualizador) e interceptación del cierre de ventana. |
| SSR, hidratación, prerendering | **No aplica** | No hay servidor. El riesgo análogo, un **destello o salto al arrancar**, se cubre verificando que la ventana se muestra solo tras aplicar tema, idioma y geometría (E2E de aplicación + revisión visual). |
| CSR | Sí | Toda la interfaz es CSR en WebView2. |
| Comportamiento sin JavaScript | **No aplica** | La WebView siempre ejecuta JavaScript. |
| Autenticación | **No aplica** a personas | Autenticación del canal con el colector (nonce efímero, versión y secuencia; NFR-006). |
| Autorización | Sí, equivalente | Lista cerrada de comandos y capacidades de Tauri; tokens de confirmación en acciones destructivas; comandos válidos solo en ciertos estados (`request_low_level_access` solo en `installable`; `install_update` solo en `verified` y sin operaciones bloqueantes); `open_external_url` solo con ID de lista cerrada. |
| Cookies y sesiones | **No aplica** | Prohibido el almacenamiento web duradero (principio X): se prueba que `localStorage`, `sessionStorage`, IndexedDB y las cookies siguen vacíos. Las «sesiones» del producto son sesiones de monitorización (FR-067), que son dominio y no autenticación. |
| Persistencia y base de datos | Sí | SQLite real en un directorio temporal por prueba. |
| Cross-browser | **Sustituido** | El único motor es WebView2 (Chromium). Firefox y WebKit no aportan cobertura. Se reemplaza por una **matriz de entorno**: Windows 11/10, temas, idiomas, tamaños, escala y temas de contraste (§ 18). |
| Responsive | Sí | Tamaños de ventana compacto, medio y expandido y escala de Windows (§ 17). |
| Tailwind | **No aplica** | Prohibido: se usan los tokens CSS del sistema de diseño (§ 20). |

### 5. Herramientas: existentes, fijadas y pendientes de evaluar

**Regla:** no se instala nada por aparecer en este documento. Antes de añadir una herramienta se comprueba si ya existe una equivalente, si es compatible con las versiones de los archivos de bloqueo, si está mantenida, si funciona en el runner Windows de CI y si su coste está justificado. Toda herramienta nueva de pruebas se registra en la tabla de versiones de la constitución (enmienda PATCH) antes de entrar en CI.

| Necesidad | Herramienta | Estado |
|---|---|---|
| Tipos y Svelte | `svelte-check`, `tsc` (`pnpm check`) | Fijada (principio XVI); existe en el harness, falta en la aplicación |
| Lint y formato | ESLint + `typescript-eslint` + `eslint-plugin-svelte`, Prettier, `rustfmt`, `clippy`, `dotnet format` y analizadores | Fijada; por configurar (T005) |
| Unitarias y componentes TS | Vitest + jsdom + Testing Library (Svelte, jest-dom, user-event) | Fijada; por configurar |
| Mocks del puente Tauri | `mockIPC` / `mockWindows` de `@tauri-apps/api/mocks` | Incluido en `@tauri-apps/api`; confirmar en la versión instalada |
| Vitest Browser Mode | — | **No se adopta de inicio**: jsdom basta para la lógica de componentes y Playwright cubre lo que exige motor real. Reevaluar solo si aparecen falsos verdes de jsdom repetidos. |
| Rust | `cargo-nextest`, `proptest`, `insta`, `cargo-llvm-cov` | Fijada |
| .NET | xUnit v3, Shouldly, coverlet, JsonSchema.Net | Fijada |
| Contratos | `jsonschema` (Rust), `ajv` (pruebas TS), Zod (puente), JsonSchema.Net (C#) | Fijada |
| E2E | Playwright (Chromium para el proyecto `frontend`; WebView2 por CDP para el proyecto `app`) | Fijada; la viabilidad CDP con Tauri se valida en un spike (T-PLAY-002) |
| Accesibilidad automatizada | `@axe-core/playwright` | Fijada |
| Mutation testing | `cargo-mutants` (Rust), StrykerJS con el runner de Vitest (TS), Stryker.NET (C#) | **Pendiente de piloto** (§ 22); no están en la constitución |
| Property-based en TS o C# | fast-check / FsCheck | **No se adoptan** salvo necesidad demostrada; en Rust se usa `proptest` |
| Benchmarks | Benchmark reproducible de `AnalysisChart` en el harness; mediciones de sesión de 1 h | Previsto (T060, T108) |

### 6. Pirámide de testing de ThrottleWatch

| Nivel | Qué valida | Dónde | Runtime | Frecuencia | Bloquea |
|---|---|---|---|---|---|
| **0 · Comprobaciones estáticas** | Tipos (`pnpm check`), lint, formato, `clippy`, analizadores .NET, compilación, igualdad de la copia del sistema de diseño (T024), igualdad de claves y ausencia de literales en catálogos (T026), validez de fixtures contra esquemas, trazabilidad requisito→test (T009a), licencias (`cargo-deny`) | Las tres pilas | Node, cargo, dotnet | Edición (lo afectado), lote, PR | Lote, PR y release. **No sustituye a ningún test** |
| **1 · Unitarias** | Reglas del motor, cálculos, normalización, validadores y esquemas, formateadores, máquinas de estados, políticas | Rust `#[cfg(test)]`, Vitest `unit`, xUnit `Unit` | Nativo / Node | Continua (tests cercanos) | Lote |
| **2 · Componentes** | Pantallas y adaptadores Svelte con el sistema de diseño: estados, interacciones, textos en ambos idiomas, semántica | Vitest `component` (jsdom) | Node + jsdom | Lote | Lote |
| **3 · Integración** | Colector falso → supervisor → validación → agregación → SQLite → eventos; comandos con servicios reales y SQLite temporal; exportación e importación; migraciones; actualizador contra servidor local; puente TS + stores + componentes con `mockIPC` | Rust `tests/`, xUnit `Integration`, Vitest `component` | Nativo / Node | Lote y PR | Lote (lo afectado), PR |
| **4 · Aceptación** | Criterios de `spec.md` e historias, y corpus etiquetado (SC-003 a SC-005, SC-016 a SC-018) | Rust `tests/acceptance_*`, más el nivel más barato de cada criterio | Mayoritariamente nativo | Lote que toca el motor; PR | Historia y PR |
| **5 · E2E (Playwright)** | Flujos críticos en navegador real: proyecto `frontend` (build de Vite con backend simulado) y proyecto `app` (aplicación Tauri de pruebas con colector replay y SQLite temporal) | `apps/desktop/e2e/` | Chromium / WebView2 | PR (smoke y afectados); `main` y programada (completa) | PR (smoke y críticos) y release |
| **6 · Transversales** | Matriz de entorno, tamaños, accesibilidad, regresión visual, rendimiento y seguridad verificable | Playwright, harness, hardware de referencia | Varios | PR según riesgo; programada; release | Release (y PR si la PR toca UI) |
| **Mutation testing** | **No es una capa**: mide si los tests de los niveles 1, 3 y 4 detectan errores | Rust, TS y C# críticos | Nativo / Node | Selectiva y programada | Solo si el plan lo exige (§ 22) |

### 7. Implementación y testing por lotes funcionales

- **Unidad de trabajo:** un lote funcional coherente (una pantalla, un caso de uso del motor, un comando con su persistencia, una vertical slice), orientativamente de 3 a 8 tareas de `tasks.md`.
- **Reducir el lote** ante: motor de diagnóstico, paradas de seguridad de la prueba guiada, protocolo y nonce, migraciones, borrado y restablecimiento transaccional, anonimización, actualizador y firma, concurrencia (supervisor, escritura por lotes) y regresiones.
- **Ampliar el lote** ante: conexión de componentes presentacionales del sistema de diseño, adaptadores simples y refactorización interna sin cambio de contrato.
- **Deuda temporal de tests:** solo dentro del lote activo. No se abre un lote nuevo con tests obligatorios pendientes del anterior. Una historia no se cierra con tests obligatorios pendientes. Una pantalla no está terminada porque «funciona a mano».

Lotes propuestos sobre las tareas existentes (`tasks.md`). Los identificadores de tarea no cambian; el lote solo las agrupa y fija su checkpoint:

| Lote | Tareas | Riesgo | Tests antes del código | Tests al cierre | Suite de cierre |
|---|---|---|---|---|---|
| L00 Infraestructura de pruebas | T001–T009a, T134–T138, T-COMP-001, T-TEST-003/004, T-PLAY-001/002/004 | Medio | — | Test vacío por pila en CI; check de trazabilidad | Nivel 0 completo |
| L01 Contratos y fixtures | T010–T013, T016, T143, T-UNIT-001 | Alto (contrato) | Fixtures válidos e inválidos antes de los tipos | Conformidad Rust, C# y Zod contra el mismo corpus | Contrato ×3 pilas |
| L02 Handshake, supervisor y registro del backend | T014, T015, T140, T142, T-INT-001 | Alto (seguridad, concurrencia) | Rechazo por nonce, versión, secuencia y tamaño | Caída, EOF, *backoff*, PID padre ausente | Integración IPC |
| L03 Colector real mínimo y spikes | T017–T021, T-INT-005 | Medio | — | Catálogo con hardware falso; arranque real en VM sin sensores | xUnit + integración Windows |
| L04 Persistencia, reloj y sesiones | T022, T023, T047a, T-TEST-002 | Alto (datos) | Migración v1 y partición de sesiones (hueco > 60 s, 24 h) | Huecos, calidad, escritura por lotes | Almacenamiento + sesiones |
| L05 Shell, puente, catálogos y diseño | T024–T026, T139, T141, T144, T150, T-COMP-002/003, T-PLAY-005 | Medio | Resolución de idioma (FR-039) | Igualdad de copia, catálogos, navegación del shell | Nivel 0 + componentes shell |
| L06 Onboarding | T079–T082 | Medio | — | Recorrido, omisión, reanudación, novedades | Componentes + E2E `frontend` |
| L07 Tema, movimiento, vidrio y ventana | T083–T087, T131 | Medio | — | Tema en caliente, geometría, barra propia, contraste con vidrio `off` | Componentes + E2E `app` (ventana) |
| L08 Normalización (US1) | T027–T029, T028a–e | Alto | Selección de temperatura representativa, P/E/LP, nivel A/B/C | Normalizador Intel/AMD/legado/híbrido | xUnit + Rust unitarias |
| L09 Ahora y cobertura (US1) | T030–T035 | Medio | — | Estados A/B/C, obsoleto, desconectado, híbrido | Componentes + E2E smoke |
| L10 Ventanas y mesetas (US2) | T036, T037a, T037b, T038–T040a | **Muy alto** | **TDD obligatorio** (constitución XIII) | — | Unitarias del motor |
| L11 Clasificador y eventos (US2) | T037c, T041, T042 | **Muy alto** | **TDD obligatorio** | Determinismo + propiedades | Unitarias + propiedades |
| L12 Narrativa, UI y corpus (US2) | T037, T043–T045 | Alto | — | Corpus etiquetado y degradado (SC-003…SC-018) | Aceptación de corpus |
| L13 Potencial (US3) | T046–T052 | **Muy alto** | **TDD obligatorio** (fórmula, cotas, redondeo) | Negativas de T052 | Unitarias + propiedades + aceptación |
| L14 Prueba guiada: estados y paradas (US4) | T055–T057, T059 | **Muy alto (seguridad)** | **TDD** de paradas FR-085 y cancelación | Watchdog, suspensión y ventana oculta | Unitarias + integración |
| L15 Prueba guiada: generador e interfaz (US4) | T056a, T058 | Alto | — | Rendimiento medido; `Detener ahora` en todas las pantallas | Integración + E2E `app` |
| L16 Análisis (US5) | T060–T066 | Medio | Agregación (extremos, huecos, bordes de eventos) | Teclado del gráfico, tabla alternativa, visual | Componentes + E2E `frontend` + visual |
| L17 Bandeja y alertas (US6) | T067–T072 | Alto | Reglas de alerta (90 s, 30 min, silencio, `below_base`) | Ciclo de vida, pausa, menú | Unitarias + integración con puertos |
| L18 Exportación, importación y sesiones (US7, HU-18) | T073–T078, T147, T148 | **Muy alto (privacidad)** | **TDD** de la lista permitida de anonimización | Ida y vuelta, migración, reevaluación | Integración + propiedades + E2E `app` |
| L19 Ajustes y ciclo de vida (US10) | T088–T094, T149, T-INT-003 | Alto (acciones destructivas) | Corrección atómica FR-061; transacciones FR-063 | Dependencias visibles, confirmaciones | Integración + componentes + E2E `app` |
| L20 Actualizador (US11) | T095–T103, T-INT-004 | **Muy alto (seguridad y red)** | Máquina de estados y tráfico cero | Firma inválida, parcial, bloqueo | Integración con servidor local + E2E `app` |
| L21 Endurecimiento | T104–T117, T-E2E-13, T-E2E-14, T-QUAL-005 | Alto | Recuperación de base dañada y disco lleno | Matriz completa, rendimiento, hardware | Nivel 4 completo |

### 8. Plan previo de cada lote

Antes de implementar un lote se añade su plan en `tasks.md`, bajo el encabezado del lote. No es obligatorio crear ya los ficheros de test. Plantilla:

```markdown
#### Plan de pruebas — L11 Clasificador y eventos
- Historia / requisitos: HU-02 · FR-009, FR-010, FR-069, FR-070, FR-078, FR-080, NFR-009
- Objetivo y funcionalidad: tabla ordenada de clasificación, gravedad, confianza con techo, eventos
- Código: backend `diagnostics/classifier.rs`, `diagnostics/events.rs` · interfaz: ninguno
- Comportamientos observables: clase, gravedad, confianza, evidencias, alternativas, intervalo
- Estados y errores: nivel A/B/C, reloj `substitute`, datos ausentes → `indeterminate`
- Casos límite: filas contiguas, 97 % de la base durante 30 s, PROCHOT sin THERMAL
- Riesgos: falso «confirmada», inversión térmica↔potencia, no determinismo
- Unitarias: positivo y negativo por fila; precedencia; techos por nivel (T037c)
- Propiedades: misma traza → mismo resultado; nivel B/C nunca `mixed_limit`
- Componentes / integración / aceptación / E2E: — / eventos persistidos en SQLite temporal / 3 trazas del corpus / ninguno
- Datos y fakes: `TraceBuilder`, fixtures `intel-external-prochot`, `zen4-thermal-by-design`
- Entorno: nativo, sin base de datos salvo el test de persistencia de eventos
- Suite de cierre: `cargo nextest run -E 'test(/classifier|events/)'` + aceptación afectada
- Criterios del checkpoint: § 35 + mutation piloto sobre `classifier.rs` (§ 22)
```

### 9. Cuándo se escriben los tests

- **Antes del código (TDD selectivo, obligatorio donde la constitución lo exige):** reglas del motor, paradas de seguridad, cálculo de potencial, anonimización, validación del protocolo y del nonce, migraciones, transacciones de borrado y restablecimiento, máquina de estados del actualizador y cualquier contrato ambiguo.
- **Inmediatamente al corregir un defecto:** test de regresión que falle sin la corrección (principio XIII).
- **Al cierre del lote:** componentes, pantallas y estados, navegación, integración de adaptadores, aceptación de flujos de interfaz, tamaños de ventana, capturas visuales y E2E.
- **Nunca al final del proyecto:** no se acumulan lotes sin tests ni se dejan los E2E para la fase de endurecimiento; cada historia cierra con sus E2E críticos.

### 10. Qué no se prueba y cómo evitar el acoplamiento

- No se genera un test por fichero, función, componente, store, rune, comando o criterio de aceptación. Se agrupan comportamientos relacionados.
- Se prueban comportamientos observables y contratos públicos: clase y evidencias devueltas, props que recibe un componente, textos y estados visibles, eventos emitidos, filas persistidas y ficheros exportados.
- No se prueban variables privadas, *helpers* internos, número de renderizados o actualizaciones reactivas, clases CSS ni orden del DOM irrelevante.
- Los componentes del sistema de diseño no se vuelven a probar dentro de la aplicación: la aplicación prueba **cómo los conecta** (adaptadores y props). Sus tests propios viven en `design/` (T-COMP-004).
- No se reimplementa el algoritmo esperado dentro del test: se usan valores calculados a mano, tablas de `spec.md` o trazas etiquetadas.

### 11. Unit testing

| Área | Pila y ubicación | Qué se prueba | Riesgo cubierto | Dobles |
|---|---|---|---|---|
| Ventanas, mesetas y núcleos activos | Rust `diagnostics/windows.rs`, `thermal.rs`, `power.rs`, `platform.rs` | Umbral 80 %, inicio de carga, ventana de turbo `max(Tau, 60 s)`, `turbo_end`, ventana 60/10 s, ocupación de razones, mesetas, bajada progresiva del límite | Confundir fin de turbo con calor; atribución errónea | `TraceBuilder`, reloj falso |
| Clasificador y confianza | Rust `diagnostics/classifier.rs` | Tabla ordenada de `spec.md`, gravedad, techos por nivel, alternativas | Falso «confirmada», recomendación equivocada | Fixtures de ventana |
| Potencial | Rust `diagnostics/potential.rs` | Fórmula de techo de potencia, cota por turbo, rango `[0,5·g, 1,0·g]`, redondeo hacia fuera a 5 %, tramos, sin cifra sin PL1 | Cifra engañosa | Valores calculados a mano |
| Sesiones y alertas | Rust | Partición por hueco > 60 s y 24 h; persistencia ≥ 90 s, enfriamiento 30 min, silencio, solo `below_base` | Alertas molestas o perdidas | Reloj falso |
| Agregación temporal | Rust `telemetry/` | Conserva mínimo, máximo, huecos, peor calidad y bordes de eventos | Gráfico que miente | Series sintéticas |
| Preferencias y dependencias | Rust | Corrección atómica FR-061; dependencias FR-062; valores de fábrica | Configuración incoherente | SQLite temporal o fake |
| Anonimización y registros | Rust `export/`, envoltorio de registros | Lista permitida; redacción de rutas, nombre de equipo, serie, MAC/IP, GUID de plan | Fuga de identificadores | Fixtures con identificadores sembrados |
| Actualizador | Rust | Estados `idle → … → installing`, bloqueos, descarte de parciales | Instalar sin firma o durante una prueba | Puerto HTTP falso |
| Normalización de sensores | C# `Normalization/` | Selección de temperatura (`package`/`Tdie` → máx. núcleos → `Tctl` con offset → sin offset), unidades, NaN/infinito → ausente | Temperatura equivocada | `FakeHardware`, `FakeSensor` |
| Topología y lecturas de bajo nivel | C# | P/E/LP desde datos CPUID falsos; bits de razón y limpieza solo de la lista cerrada | Grupos erróneos; escritura indebida | Proveedor de registros falso |
| Puente y configuración | TS `lib/bridge/`, `lib/config/env.ts` | Esquemas Zod (aceptan válidos y rechazan inválidos), resultado discriminado, errores estructurados | Datos corruptos en pantalla | Fixtures de contrato |
| Formato e idioma | TS | Resolución `es/ca/gl/eu/ast/an → es`, resto → `en` (incluidos `pt` y `ca-AD`); números y fechas por idioma (`98,5 °C` / `98.5 °C`) | Idioma o formato incorrecto | Tablas de casos |
| Enrutado del shell | TS | Onboarding pendiente, reanudado u omitido → destino correcto | Arranque en pantalla equivocada | Estado de onboarding simulado |
| Envoltorio de registros | TS / Rust / C# | Niveles, redacción, agrupación de repetidos, límite de reenvío, formato `Europe/Madrid` con cambio de hora | Fuga de datos, ruido | Reloj falso, sumidero en memoria |

Reglas: sin Internet, sin credenciales reales, sin fecha real, sin aleatoriedad no controlada, sin esperas reales y sin dependencia entre tests.

### 12. Component testing de Svelte

- **Entorno:** Vitest con proyecto `component` en jsdom y `@testing-library/svelte`. El plugin de Svelte debe resolver la condición `browser` para Svelte 5; se valida en T-COMP-001.
- **Qué se prueba:** adaptadores de cada feature que conectan componentes del sistema de diseño con datos del puente: `Ahora` (StatusHero, ContextStrip, CoverageMatrix), `Informe`, `Sesiones`, `Diagnóstico guiado`, `Ajustes`, `Análisis` (sin geometría del SVG), banner global, onboarding y diálogos de cierre, exportación e importación.
- **Qué se comprueba:** renderizado por estado (carga, vacío, degradado, error, obsoleto, desconectado), textos en **ambos idiomas**, controles habilitados o deshabilitados **con su motivo visible** (FR-062), callbacks al puente con parámetros validados, foco inicial de diálogos, `Esc` que cierra sin guardar (FR-046) y ausencia de cifras sin método (SC-002).
- **Qué no se comprueba en jsdom** (va a Playwright): maquetación, `backdrop-filter`, contraste real, foco visible pintado, desplazamiento y capturas.
- **Dobles:** `mockIPC` o un `FakeBridge` tipado con los mismos esquemas Zod; *builders* `liveSnapshot()`, `coverage()`, `report()`, `session()` y `preferences()`.
- No se duplica en la interfaz lo que ya prueban las unitarias Rust: el componente recibe una clase ya calculada y solo se verifica cómo la presenta.

### 13. Contrato de testabilidad

1. **Selección:** rol accesible → nombre accesible → etiqueta → texto visible estable **del catálogo activo** → estado semántico (`aria-pressed`, `aria-expanded`, `aria-disabled`) → `data-testid` solo si lo anterior no basta.
2. **Textos:** se consultan a través del catálogo (`t('key')`) para que el mismo test funcione en español e inglés; nunca se escriben literales en el test.
3. **`data-testid` permitidos** (nombres de dominio estables): `app-root`, `title-bar`, `global-banner`, `context-strip`, `status-hero`, `coverage-matrix`, `analysis-chart`, `core-map`, `guided-stop`, `bottom-bar`, `more-menu`, `loading-state`, `empty-state`, `error-state`.
4. **Obligaciones para el código:** todo control interactivo tiene nombre accesible; todo campo tiene etiqueta; todo error importante es localizable por rol (`alert` o `status`) o por texto; los controles exponen su estado.
5. **Prohibido:** posiciones, índices `nth()` innecesarios, DOM profundo, clases CSS e identificadores generados.

### 14. Integration testing

#### 14.1 Canal con el colector (equivalente a una API externa)

- **Qué:** colector falso/replay (T016) como proceso real → supervisor → validación JSON Schema + serde → agregación → eventos.
- **Casos:** handshake correcto; nonce, versión, secuencia o tamaño incorrectos → rechazo y cierre tras errores repetidos; EOF; cuelgue (sin latido); reinicio con *backoff* limitado y petición de intervención (NFR-004, SC-010); secuencias perdidas; muestras con NaN o fuera de rango → ausentes; PID padre ausente → el colector termina.
- **Runtime:** nativo; sin hardware; sin base de datos salvo el test de extremo a extremo del pipeline.
- **Suite:** `integration-ipc`; lotes L01, L02 y L08; bloquea PR.

#### 14.2 Comandos y eventos Tauri (equivalente a endpoints)

- Se prueban los manejadores con los servicios reales y SQLite temporal, **sin WebView**: los comandos son adaptadores delgados (principio IX), así que el test llama al servicio de aplicación con los mismos DTO serde que usa el comando.
- **Por cada comando:** entrada válida; entrada inválida (tipo, rango, campo desconocido) → error estructurado con `code`, `path` y `message_key`; estado no permitido; sin efectos parciales.
- **Comandos con restricciones de estado:** `delete_session` (no la activa), `set_session_reference` (solo guiadas), `start_guided` (`blocked_on_battery`, `already_running`, `operation_in_progress`), `request_low_level_access` (solo `installable`), `install_update` (solo `verified`, bloqueos), `open_external_url` (lista cerrada) y `reevaluate_report` (solo importadas).
- **Contrato con la interfaz:** los DTO serializados por Rust se validan con los esquemas Zod del puente mediante fixtures generados por los tests Rust (*golden files* en `packages/contracts/fixtures/app/`).

#### 14.3 Formularios y ajustes (equivalente a Server Actions)

| Nivel | Qué demuestra |
|---|---|
| Unitaria TS | Esquema Zod del formulario: obligatorios, límites (horas del periodo de silencio), mensajes por código |
| Unitaria o integración Rust | Revalidación, dependencias FR-061/062, persistencia y respuesta con correcciones aplicadas |
| Componente | Estado pendiente, error mostrado, valores conservados, control deshabilitado con motivo |
| E2E `app` (solo flujos críticos) | Borrar datos frente a restablecer; bandeja → corrección de inicio oculto; retención «solo esta sesión» |

Doble envío: dos pulsaciones sobre `Exportar`, `Descargar`, `Iniciar prueba` o `Eliminar todos mis datos` producen una sola operación (prueba de integración con dos llamadas concurrentes).

#### 14.4 Persistencia SQLite

- **Base real en directorio temporal por test** (`tempfile`), en modo WAL y con claves foráneas; nunca la base del perfil del desarrollador.
- **Casos:** inserción por lotes; consultas de sesión y ventana de análisis; retención (1, 7 y 30 días y «solo sesión» al salir); borrado de datos frente a restablecimiento (FR-050, FR-051, FR-063) con fallo inyectado a mitad; restricciones y claves foráneas; migraciones desde **cada** versión anterior con fixtures de base congelados; disco lleno (puerto de almacenamiento con fallo inyectado → modo solo memoria y reintento cada 60 s); base dañada (fichero corrupto → apartado `.corrupt-<fecha>`, base nueva, oferta de exportar; nunca borrado); exportación desde instantánea con escrituras concurrentes.
- Un fake en memoria solo sustituye a SQLite en unitarias de servicios que no prueban persistencia.

#### 14.5 Exportación, importación y reproducción

- CSV (coma, punto decimal, UTC, claves inglesas) e informe JSON validado por esquema.
- **Ida y vuelta:** exportar → importar → reproducir sin sensores → mismo diagnóstico (FR-027, NFR-009).
- Importación de versión anterior (migrada), futura (`import.schema_too_new`), corrupta y truncada.
- Reevaluación con las reglas actuales junto a la original; una importada nunca actúa como referencia local (FR-073).
- **Fuga de identificadores (SC-008):** exportación anónima de trazas sembradas con nombre de equipo, usuario, serie, MAC, IP, rutas y GUID de plan → búsqueda automática de todos ellos en el resultado.
- El diálogo nativo de guardar o abrir se sustituye por un **puerto de diálogo falso** que devuelve una ruta temporal.

#### 14.6 Autenticación del canal y autorización de comandos

- **Canal con el colector:** nonce efímero obligatorio; un mensaje sin nonce o con uno ajeno se rechaza y se registra sin el valor.
- **Superficie de la WebView:** test que compara la lista de comandos registrados y los permisos de `capabilities/` con los de `contracts/application-commands.md`; un comando nuevo sin contrato hace fallar el test.
- **Acciones destructivas:** sin `confirmation_token` válido y vigente → rechazo.
- **Rutas:** ningún comando acepta rutas desde la WebView (las rutas las decide Rust mediante el diálogo nativo).

#### 14.7 Actualizador

- Servidor HTTP local en el test con manifiesto y artefactos: versión disponible, sin versión, manifiesto inválido, firma inválida, descarga parcial o interrumpida → descarte, y firma válida → `verified`.
- **Clave de prueba** generada para tests; nunca la clave real (principio XV).
- **Tráfico cero con el actualizador apagado:** el puerto HTTP cuenta llamadas y el test exige cero durante una sesión simulada completa (FR-052, SC-014, puerta 12).
- Cadencia de 24 h con reloj falso; la comprobación manual ignora la cadencia; instalación bloqueada durante prueba guiada, exportación, importación o borrado (FR-057).
- El endpoint solo es sustituible en compilaciones de test (`cfg(test)` o característica `e2e`), nunca en producción.

#### 14.8 Ciclo de vida, ventana y bandeja (puertos nativos)

Rust expone puertos para ventana, bandeja, notificaciones, autostart, monitores, energía, idioma y tema de Windows. Las pruebas de integración verifican con fakes: decisión de la primera X (`exit`, `tray`, `dismiss` sin persistir); bloqueo de cierre con prueba, exportación, descarga o instalación; geometría fuera de monitores activos → recentrado en el principal; no se persiste el minimizado; instancia única → enfocar la existente; reanudación desde suspensión → hueco, redescubrimiento y sesión nueva (FR-065); cambio de alimentación aplicado en la siguiente muestra (FR-060); notificaciones con persistencia, enfriamiento y silencio; estados del icono de bandeja (FR-059).

#### 14.9 Colector .NET con LibreHardwareMonitorLib real

Suite `Integration` de xUnit que solo se ejecuta en Windows: arranca el colector real en el runner de CI, que es una VM **sin sensores**, y verifica un catálogo degradado coherente (`virtualized-no-sensors`), la ausencia de excepciones y el cierre limpio por EOF. Es el único test automático con la biblioteca real; el resto usa fakes.

### 15. Acceptance testing

- Cada criterio de aceptación (HU) y cada FR/NFR/SC se enlaza en `traceability.md` (T009a) con el test que lo demuestra o con «manual» y su motivo. CI falla si falta una entrada.
- Los nombres de test incluyen el identificador para facilitar la búsqueda: `fr069_prochot_without_thermal_is_never_confirmed` (Rust), `describe('HU-09 · FR-039 resolución de idioma')` (Vitest) y etiquetas `@HU-04 @FR-085` (Playwright).
- Se usa estructura Given/When/Then en el nombre o en comentarios del test; **no se introduce Cucumber**.
- **Aceptación del motor por corpus** (`tests/acceptance_corpus.rs`): reproduce las trazas etiquetadas y sus copias degradadas y calcula las métricas SC-003, SC-004, SC-005, SC-016, SC-017 y SC-018. El umbral de cada SC es la aserción. Se ejecuta en cada lote que toca `diagnostics/` o `ruleset-*.json` y en toda PR.

### Tests funcionales derivados de historias de usuario

Leyenda: **N** = requiere navegador · **S** = requiere backend real · **BD** = requiere SQLite · **E2E** = requiere Playwright. Prioridad: P1 (bloquea historia y PR), P2 (bloquea historia), P3 (bloquea release).

| HU | Criterios agrupados (comportamiento observable) | Riesgo | Nivel más barato | Datos y dobles | Lote · suite | N/S/BD/E2E | Prioridad |
|---|---|---|---|---|---|---|---|
| HU-01 | Conclusión, CPU, temperatura, carga, frecuencia activa frente a base, potencia y frescura; «No disponible» para ausentes; P/E/LP; franja con datos obsoletos a los 5 s; «Datos insuficientes» con colector desconectado; frecuencia estimada o sustituta rotulada | Mostrar cero o datos viejos como actuales | Unitaria Rust (snapshot y frescura) + componente (`Ahora`, `ContextStrip`, `CoverageMatrix`) | `intel-hybrid-idle-cores`, `derived-clock-only`, `missing-effective-clock`, `collector-disconnect`, `virtualized-no-sensors`; reloj falso | L08, L09 · unit + component; smoke E2E | Solo smoke / S en E2E / — / smoke | P1 |
| HU-02 | «Confirmada» exige THERMAL; PROCHOT solo no es calor; mesetas → «compatible»; fin de turbo normal; chasis recomienda ventilar; `boost` dentro de especificación y `below_base` problema; potencia no recomienda refrigeración; juego de pocos núcleos; evidencias, ausencias, alternativas, intervalo y confianza; nivel de detalle y ganancia del acceso avanzado; batería, plan y EcoQoS como alternativas | Diagnóstico falso o recomendación equivocada | Unitaria Rust + **aceptación por corpus** + componente del adaptador de evidencias | Corpus etiquetado y degradado; `intel-pl2-tau-drop`, `laptop-dptf-chassis`, `intel-external-prochot`, `zen4-thermal-by-design`, `game-few-cores`, `ecoqos-efficiency`, `battery-power-plan-change` | L10–L12 · unit, acceptance-corpus, component | — / — / — / — | P1 |
| HU-03 | Estimación solo con límite de potencia legible; tramo y rango a múltiplos de 5 sin decimales; potencia → la refrigeración apenas influye; sin datos → sin cifra y motivo; guiada mide trabajo y desglosa; antes/después solo entre guiadas comparables; ni tiempo en throttling ni turbo de caja como referencia; híbridas por grupos activos | Cifra engañosa | Unitaria Rust + propiedades + aceptación; componente de `Informe` | Trazas nivel A y degradadas; `guided-standard-intel` | L13, L15 · unit, property, acceptance, component | — / — / BD para referencias / — | P1 |
| HU-04 | Lista previa de lo que ocurrirá; fases con tiempo restante y lecturas; duraciones 3/4/6 min; `Detener ahora` siempre visible con atajo y barra global, `Esc` no detiene; cancelar conserva parcial; paradas FR-085 (no por alcanzar el límite); ocultar o suspender cancela; batería avisa o bloquea; notificación al terminar; ofrecer referencia; nunca inicia desde onboarding | **Daño o carga huérfana** | **TDD** unitaria Rust de paradas + integración del controlador con generador falso + componente + E2E `app` | Generador falso (en CI **nunca** carga real); trazas de sobretemperatura, sensor perdido y latido perdido | L14, L15 · unit, integration, component, e2e-app | Sí / Sí / Sí / **Sí** | P1 |
| HU-05 | Pistas con cursor y eje comunes; eventos con tipo, duración y evidencia; huecos como ausencia; mapa de núcleos con alternancia sin perder selección; alternativa textual, teclado y tabla | Gráfico que miente o es inaccesible | Unitaria Rust (agregación) + componente + E2E `frontend` (teclado y visual) | Ventanas de análisis con huecos y eventos; alta densidad | L16 · unit, component, e2e-frontend, visual | Sí / — / — / Sí | P2 |
| HU-06 | Bandeja solo si se habilita; notificaciones apagadas de fábrica y botón de prueba; cuatro avisos con 90 s, 30 min y silencio; pulsar aviso térmico abre `Análisis` en ese momento; cinco estados de icono; menú; comportamiento en batería | Avisos molestos o perdidos | Unitaria Rust (reglas) + integración con puertos de bandeja y notificación + componente | Trazas persistentes y picos breves; reloj falso | L17 · unit, integration | — / Sí / — / — (manual de release para lo nativo) | P2 |
| HU-07 | CSV + JSON versionado desde sesión, informe o tramo con el mismo diálogo; vista previa de campos, tamaño y nombre; anonimización por lista permitida; importar con validación, migración o avisos; reevaluación junto a la original; reproducir sin sensores; formato técnico inglés, coma, punto y UTC | **Fuga de datos**; fichero incoherente | **TDD** unitaria de anonimización + integración ida y vuelta + propiedades + E2E `app` del diálogo | Trazas con identificadores sembrados; ficheros de versiones anteriores, futura y corrupta; puerto de diálogo falso | L18 · unit, property, integration, e2e-app | Sí / Sí / Sí / Sí | P1 (privacidad) |
| HU-08 | Cinco diapositivas; glosario; omitir sin confirmación desde las cuatro primeras; reanudar y repetir desde Ayuda; detección pasiva sin UAC en la quinta; cobertura parcial continúa; cinco estados del acceso avanzado y solo «instalable» ofrece instalar; `Abrir Ahora` principal; novedades con «Entendido» | Permisos inesperados; bloqueo en primer inicio | Unitaria (enrutado, estado versionado) + componente + E2E `frontend` por teclado (SC-011) | Perfiles: limpio, a medias, completado, versión anterior | L06 · unit, component, e2e-frontend | Sí / — / — / Sí | P1 |
| HU-09 | `Sistema` inicial; locales ibéricos → español, resto (incluidos `pt` y `ca-AD`) → inglés; selección manual prevalece; cambio de Windows en el siguiente inicio; primer idioma de visualización; catálogos completos; formato por idioma y solo °C | Interfaz ilegible para el usuario | **Unitaria** (tabla de locales, SC-012) + check de catálogos (nivel 0) + componente en ambos idiomas | Tabla de locales; catálogos | L05, L07 · unit, static | — / — / — / — | P1 |
| HU-10 | Tema `Sistema/Claro/Oscuro` en caliente; contraste en ambos; movimiento `Sistema/Reducido/Completo`; vidrio legible, reducible y degradado automático; aspecto nativo (sin subrayado ni cursor de mano salvo el gráfico); animaciones solo en transiciones; 200 % sin ocultar acciones | Ilegibilidad, rendimiento, aspecto web | Componente (atributos `data-theme/motion/glass`) + E2E `frontend` con `getComputedStyle` + axe + visual | Eventos simulados de tema, transparencia y movimiento de Windows | L07 · component, e2e-frontend, visual, a11y | Sí / — / — / Sí | P1 |
| HU-11 | Barra propia completa; doble clic; controles accesibles también en onboarding y errores; geometría recordada; mínimo 480×600 e inicial 1100×760 centrada; monitor desaparecido; instancia única; cierre durante prueba, exportación, descarga o instalación | Ventana perdida o cierre destructivo | Integración Rust con puertos (geometría, instancia, bloqueo) + E2E `app` (barra y doble clic) | Monitores simulados; geometrías guardadas fuera de pantalla | L07, L19 · integration, e2e-app; varios monitores reales: manual | Sí / Sí / Sí / Sí | P1 |
| HU-12 | Primera X pregunta, guarda y señala Ajustes; bandeja activa monitorización; `Esc` no guarda; cierres posteriores aplican la decisión; en Ajustes sin selección «Se te preguntará al cerrar»; inicio con Windows apagado, con ventana u oculto (oculto exige bandeja); desactivar bandeja corrige en el mismo gesto y lo dice | Proceso oculto no deseado | Unitaria/integración Rust (FR-046, FR-061) + componente de Ajustes + E2E `app` de la primera X | Preferencias `unset`, `exit` y `tray` | L19 · integration, component, e2e-app | Sí / Sí / Sí / Sí | P2 |
| HU-13 | Grupos de Ajustes; perfiles con números solo en avanzado; retención de 4 opciones con 7 días inicial y espacio visible; avanzado plegado la primera vez; dependientes visibles con motivo; Acerca de (ayuda, documentación en línea con aviso, licencias, resumen técnico sin identificadores, carpeta de registros); ninguna opción envía datos | Configuración confusa; fuga en el resumen | Componente + unitaria Rust (resumen técnico sin identificadores) | Preferencias de fábrica | L19 · component, unit | — / — / — / — | P2 |
| HU-14 | Eliminar datos conserva preferencias, onboarding y geometría; restablecer borra todo, desregistra el inicio y elimina descargas; siguiente arranque como instalación nueva; transaccional con fallos parciales comunicados; desinstalar pregunta | **Pérdida de datos** | **TDD** integración Rust con SQLite y fallos inyectados + E2E `app` (confirmaciones diferenciadas) | Base poblada; fallo inyectado a mitad | L19 · integration, e2e-app; desinstalador: manual de release | Sí / Sí / Sí / Sí | P1 |
| HU-15 | Apagado = cero tráfico; cada 24 h como máximo; manual antes; solo estable; disponible avisa sin descargar; `Descargar` con progreso y verificación; `Instalar` separado, anuncia cierre y rechaza inválidos; bloqueos con motivo; fallos aislados; endpoint fijo | Instalación no firmada; red no consentida | Unitaria (estados) + integración con servidor local + E2E `app` del flujo en Ajustes | Servidor de releases local; clave de prueba | L20 · unit, integration, e2e-app | Sí / Sí / Sí / Sí | P1 (seguridad) |
| HU-16 | Todo sin Internet; sin telemetría; retención y espacio visibles; anonimización descrita; única red: actualizador; documentación en línea solo a petición; resumen sin identificadores | Pérdida de confianza | Integración (tráfico cero) + E2E: cualquier petición fuera del origen de la aplicación falla el test | Todas las suites E2E con guardia de red | Transversal · todas | — | P1 |
| HU-17 | Componentes y vocabulario coherentes; ambos idiomas, temas y tamaños; estados de carga, vacío, degradado y error; teclado, foco y no solo color; gráficos accesibles; barra coherente; navegación compacta con `Más`; banner global; atajos `Ctrl+1…6`, `Ctrl+,`, `Ctrl+Shift+X`, `Ctrl+E`, `F1` | Incoherencia, inaccesibilidad | Nivel 0 (copia de diseño, catálogos) + E2E `frontend` (atajos, navegación compacta, axe) + visual | Mock de backend por escenario | Transversal · e2e-frontend, a11y, visual | Sí / — / — / Sí | P1 |
| HU-18 | Lista con estados, fecha, duración, resultado y «Referencia»; partición automática de sesiones pasivas; informe provisional en curso y congelado al cerrar; abrir, exportar y eliminar con confirmación salvo la activa; «solo esta sesión» borra al salir | Historial incoherente | Unitaria/integración Rust (FR-067, congelado) + componente `Sesiones` + E2E `app` | Sesiones de todos los tipos; reloj falso | L04, L19 · integration, component, e2e-app | Sí / Sí / Sí / Sí | P2 |
| HU-19 | Colector caído: interfaz conservada, datos obsoletos, reintentos limitados y resumen técnico; disco lleno: memoria, aviso persistente y reintento; base dañada: apartar, crear, ofrecer exportar; fallos del actualizador, exportación o importación contenidos; barra global con reintentar y resumen | Pantalla en blanco o pérdida de datos | Integración Rust con fallos inyectados + componente (banner) + E2E `app` de colector caído | Colector falso que muere; puerto de almacenamiento con fallo; base corrupta | L02, L21 · integration, component, e2e-app | Sí / Sí / Sí / Sí | P1 |

Casos transversales que cada historia revisa al planificar su lote: camino correcto, validación, vacío, error recuperable y no recuperable, reintento, cancelación, límites, datos incorrectos o manipulados, tamaños de ventana e idioma. «Sesión inexistente» y «usuario no autorizado» se interpretan como sesión de monitorización inexistente y comando no permitido en el estado actual.

### 16. Playwright

#### 16.1 Dos proyectos, dos propósitos

| Proyecto | Qué ejecuta | Backend | Para qué | Cuándo |
|---|---|---|---|---|
| `frontend` | Build de producción de Vite servido con `vite preview` en Chromium | **Simulado**: `mockIPC` con escenarios tipados y validados con los esquemas Zod | Navegación, teclado, accesibilidad, tamaños, temas, idiomas, capturas visuales, atajos y gráfico | Rápido; PR (afectados y smoke) y `main` (completo) |
| `app` | Aplicación Tauri compilada con la característica `e2e` y conectada por CDP a WebView2 | **Real**: backend Rust, SQLite temporal, colector replay, generador de carga falso, puertos de diálogo y red de prueba | Integración interfaz-backend, persistencia, ventana, ciclo de vida, resiliencia y flujos críticos | PR (smoke + afectados), `main`, programada y release |

- La compilación `e2e` es la única que activa el puerto de depuración remota de WebView2 y las variables `TW_DEV_*` (directorio de datos temporal, traza replay, inyección de fallos, endpoint del actualizador de prueba). **Nunca se distribuye** (principios XV y XVII).
- Cada flujo documenta si usa backend simulado, backend real con SQLite temporal, colector replay, puerto de diálogo falso o servidor de releases local. No se mezclan estrategias dentro de un mismo test.
- No se usa Playwright para lógica que ya demuestran las unitarias o la integración.

#### 16.2 Smoke (`@smoke`)

Sobre el proyecto `app` con la traza `intel-normal` y sobre `frontend` con el escenario por defecto:

- la aplicación arranca y la ventana no queda en blanco (`app-root` y `title-bar` visibles);
- en un perfil limpio aparece el onboarding, y con onboarding completado aparece `Ahora` con conclusión principal;
- el colector pasa a `running` y la primera muestra se ve (presupuesto orientativo de NFR-002 y NFR-010, que en CI **avisa** en lugar de bloquear porque las VM varían);
- funciona la navegación básica (`Ctrl+1` a `Ctrl+6`);
- no hay excepciones de página, `console.error` ni errores de red (§ 16.3);
- el almacenamiento web sigue vacío (principio X).

#### 16.3 Errores de navegador y red

- Una *fixture* común registra `pageerror`, `console` (`error` y `warning`), `requestfailed` y respuestas inesperadas, y falla al terminar el test si aparece algo no declarado.
- **Allowlist inicial vacía:** la aplicación no carga recursos de terceros ni usa `console.*` (principio XVII). Toda entrada nueva en `e2e/support/console-allowlist.ts` lleva justificación e incidencia.
- Un test que provoca un error esperado (por ejemplo, la caída del colector) **declara los códigos de evento** que espera; el resto sigue fallando.
- **Guardia de red:** cualquier petición a un origen distinto del de la aplicación hace fallar el test (HU-16), salvo en los tests del actualizador contra su servidor local.
- Se distingue error funcional (falla el test), error de infraestructura (se reintenta el job y se registra), error del servicio simulado (falla el test: el mock está mal) y error externo (no existe en este producto).

#### 16.4 Entorno

- **Desarrollo local:** `frontend` puede apuntar al servidor de desarrollo de Vite para iterar; `app` usa la compilación de depuración con la característica `e2e`.
- **CI, PR relevantes y release:** `frontend` contra `vite build` + `vite preview`, y `app` contra la compilación de publicación con la característica `e2e`.
- El artefacto que se distribuye no se puede conducir con Playwright (no tiene depuración remota). Se valida con la verificación nativa de release (§ 31.4).
- Antes de configurar nada se inspeccionan los scripts reales, la configuración de Tauri, los puertos y las variables `TW_DEV_*` documentadas en `.env.example`.

#### 16.5 Selectores

Orden: `getByRole` → `getByLabel` → `getByText` con texto del catálogo activo → atributos ARIA → `getByTestId` (§ 13). Sin CSS profundo, posiciones ni identificadores generados.

#### 16.6 Esperas y sincronización

- Se usan los *locators* y `expect` con reintento automático, URL o pantalla activa, estados habilitados y eventos observables.
- **Prohibido** `waitForTimeout`, `sleep`, retardos arbitrarios y bucles de espera manuales.
- El tiempo de la aplicación se controla con `page.clock` (proyecto `frontend`) o con el reloj de la traza replay (proyecto `app`), no esperando tiempo real. Una prueba de 4 minutos se reproduce acelerada.
- No se suben los *timeouts* para esconder una condición de carrera: se investiga.

#### 16.7 Datos y aislamiento

- Cada test del proyecto `app` recibe su **propio directorio de datos temporal** y su proceso de aplicación (fixture de worker o de test). Nada se comparte ni se reutiliza entre tests.
- Los tests pueden ejecutarse solos, en cualquier orden y repetidos (`--repeat-each` en la ejecución programada).
- Paralelismo: `frontend` en paralelo; `app` con un worker por instancia y un directorio propio, limitado por la instancia única (identificador de aplicación distinto en la compilación `e2e`).

#### 16.8 Estados reutilizables (equivalente a `storageState`)

No hay autenticación. Su equivalente son **perfiles sembrados** que se generan una vez por ejecución y se copian a cada test:

```text
fresh-install        sin preferencias ni onboarding
onboarding-midway    onboarding interrumpido en la diapositiva 3
ready                onboarding completado, preferencias de fábrica
tray-enabled         bandeja y avisos activos
with-history         sesiones pasivas, guiadas (una de referencia) e importadas
updates-on           actualizador activado
```

Hay tests específicos del recorrido de onboarding; el resto parte de `ready` y no repite el recorrido por la interfaz.

#### 16.9 E2E derivados de historias

| ID | Historia y criterio | Por qué necesita navegador | Proyecto y backend | Perfil y traza | Tamaño de ventana | Visual | Prioridad y suite |
|---|---|---|---|---|---|---|---|
| E2E-01 | HU-01/11 arranque y smoke | Integración real de WebView, backend y colector | `app` · real + replay | `ready` · `intel-normal` | 1100×760 | No | P1 · `@smoke` |
| E2E-02 | HU-08 onboarding completo, omitido y reanudado solo con teclado (SC-011) | Foco y teclado reales | `frontend` · mock | `fresh-install`, `onboarding-midway` | 1100×760 y 480×600 | Diapositivas 1 y 5 | P1 · `@critical` |
| E2E-03 | HU-01/19 colector desconectado: banner global, «Datos insuficientes», reintento y resumen técnico (SC-010) | Banner fijo y recuperación reales | `app` · replay `collector-disconnect` | `ready` | 1100×760 | Estado desconectado | P1 · `@critical` |
| E2E-04 | HU-04 prueba guiada: consentimiento, fases, `Detener ahora` visible en otra pantalla, `Ctrl+Shift+X`, `Esc` no detiene, cancelación → incompleta, cerrar la ventana pregunta «Detener y salir» | Barra global y atajos reales | `app` · generador **falso** | `ready` · `guided-standard-intel` | 1100×760 y 480×600 | Fase de carga sostenida | P1 · `@critical` |
| E2E-05 | HU-05 análisis: cursor y rango por teclado, tabla alternativa, huecos visibles, eventos | Geometría SVG y teclado reales | `frontend` · mock con ventanas de análisis | `with-history` | 1100×760 | Sí (huecos y eventos) | P2 |
| E2E-06 | HU-18 sesiones: abrir informe, marcar referencia, eliminar con confirmación, sesión activa no eliminable | Persistencia real a través de la interfaz | `app` · real | `with-history` | 1100×760 | Lista y vacío | P2 |
| E2E-07 | HU-07 exportar e importar: vista previa, anonimizar, fichero vía diálogo falso, resultado de importación y reproducción | Diálogo, progreso y resultado integrados | `app` · real + puerto de diálogo | `with-history` | 1100×760 | Diálogo de exportación | P1 |
| E2E-08 | HU-12/13/14 ajustes: dependencias visibles con motivo, corrección de bandeja, borrar frente a restablecer | Confirmaciones diferenciadas y persistencia | `app` · real | `tray-enabled`, `with-history` | 1100×760 y 480×600 | Ajustes plegado | P1 |
| E2E-09 | HU-09/10 idioma, tema, movimiento y vidrio en caliente; temas de contraste | CSS calculado y medios emulados | `frontend` · mock con eventos de Windows | `ready` | 1100×760 | Claro/oscuro × es/en | P1 · `@a11y` |
| E2E-10 | HU-11/12 barra de título, doble clic, primera X (`Esc` no guarda); el bloqueo de cierre durante la prueba se cubre en E2E-04 | API real de ventana de Tauri | `app` · real | `ready` | 1100×760 | No | P1 |
| E2E-11 | HU-17 navegación compacta (`Más` alcanza Sesiones, Diagnóstico guiado y Ajustes) y atajos de navegación (`Ctrl+1…6`, `Ctrl+,`) | Maquetación compacta real | `frontend` · mock | `ready` | 480×600 y 840×760 | `BottomBar` | P1 |
| E2E-12 | HU-15 actualizador: apagado sin tráfico; encendido → disponible → descargar → verificada → instalar bloqueado durante la prueba | Flujo de estados completo en Ajustes | `app` · servidor de releases local | `updates-on` | 1100×760 | No | P1 |
| E2E-13 | HU-19 disco lleno y base dañada: aviso persistente, modo memoria, recuperación | Banner y continuidad reales | `app` · inyección de fallos `TW_DEV_*` | `with-history` | 1100×760 | No | P2 |
| E2E-14 | HU-17 atajos que dependen de funciones posteriores: `Ctrl+E` (exportar) y `F1` (ayuda) | Atajos globales reales | `frontend` · mock | `with-history` | 1100×760 | No | P2 |

Antes de crear cada E2E se confirma que el criterio no puede demostrarse con un test más barato (§ 0.2).

#### 16.10 Artefactos de diagnóstico

- *Screenshot* y *trace* en fallo (`retain-on-failure`); vídeo solo en el proyecto `app` y solo en fallo.
- En el proyecto `app` se adjuntan al informe los registros JSON de la ejecución (sin identificadores, principio XVII) y el resumen técnico.
- CI conserva los artefactos de fallos 14 días; los tests correctos no generan artefactos pesados.

### 17. Tamaños de ventana (responsive)

Puntos de ruptura reales del sistema de diseño (`design/tokens/tokens.ts`): compacto < 700 px, medio 700–979 px y expandido ≥ 980 px.

| Perfil | Viewport lógico | Por qué |
|---|---|---|
| Mínimo (compacto) | 480×600 | Ventana mínima (HU-11); navegación inferior y `Más` |
| Medio | 840×760 | Único tramo con navegación intermedia |
| Inicial (expandido) | 1100×760 | Tamaño de primera apertura |
| Altura útil < 500 | 480×384 maximizada | Contenido con desplazamiento; `TitleBar`, banner global y `Detener ahora` fijos (escenario 9 de la historia 9 de `spec.md`) |
| Escala 200 % | 480×600 y 480×500 (mínimo reducido de 1080p al 200 %) con `deviceScaleFactor: 2` | NFR-011, HU-10 y escenario 8 de la historia 9 de `spec.md` |

HU-11 cita el mínimo de 480×600 que pidió el cliente; el valor vigente, con sus excepciones para monitores bajos, está en `spec.md` (parámetro «Ventana»), que prevalece según el cierre de estas historias.

Se comprueba: navegación, acciones críticas visibles sin desplazamiento horizontal, formularios, menús y diálogos, tablas (CPU y alternativa del gráfico), banner global y `Detener ahora`. No se prueban resoluciones arbitrarias.

### 18. Matriz de entorno (sustituye al cross-browser)

| Momento | Matriz |
|---|---|
| Desarrollo | `frontend` en Chromium, tema claro, español, 1100×760, solo tests afectados |
| Checkpoint de lote | Igual + suite del lote |
| PR | `frontend` afectados + `@smoke`; `app` `@smoke` + afectados; si la PR toca interfaz o tokens: ambos temas × ambos idiomas en las pantallas afectadas, más 480×600 |
| `main` | E2E completos de ambos proyectos, visual y accesibilidad completos |
| Programada (nocturna) y release | Matriz completa: temas × idiomas × tamaños × escala 100/125/150/200 %, `forced-colors: active`, `reduced-motion`, niveles de vidrio `off` y `full`; proyecto `app` en Windows Server 2025 (runner) y, en release, VM limpias con Windows 11 25H2/24H2 y Windows 10 22H2 |

Firefox y WebKit no se ejecutan: no forman parte del runtime real.

### 19. Accesibilidad

- **Automática (bloquea PR si la PR toca interfaz):** axe sin infracciones `serious` o `critical` en cada pantalla y estado de la matriz afectada (puerta 5); roles, nombres, etiquetas y estados verificados por las propias consultas de Testing Library y Playwright.
- **Teclado (E2E):** recorridos completos de onboarding (SC-011), ajustes, prueba guiada y análisis; orden lógico; sin trampas de foco; foco visible y nunca oculto bajo la barra o el banner; `Esc` y atajos según HU-17.
- **Regiones vivas:** cambios de conclusión con `polite`; paradas de seguridad y pérdida del colector con `assertive` (principio XII).
- **Contraste:** el cálculo de contraste de axe más una prueba específica de texto sobre vidrio en todos los niveles (FR-043b); `forced-colors: active` emulado.
- **Movimiento:** con `reduced-motion`, ninguna animación tiene duración mayor que cero.
- **Manual (release):** Narrador y NVDA sobre los flujos P1, con la versión registrada. Una prueba automática no sustituye a una auditoría.

### 20. Contrato visual y regresión visual

**Contrato de estilos** (E2E `frontend`, rápido, sin capturas):

- `tokens.css` está cargado: `getComputedStyle(document.documentElement)` devuelve valores no vacíos para los tokens críticos de color, superficie, vidrio y movimiento.
- Cambiar `Claro ↔ Oscuro` cambia el fondo y el color del texto de la superficie principal.
- `data-glass="off"` deja `backdrop-filter: none`.
- Ningún botón usa `cursor: pointer` ni muestra subrayado al pasar el puntero o enfocar (NFR-015), salvo la superficie de trazado de `Análisis`.
- No se reproduce el sistema de diseño en tests ni se depende de nombres de clases.

**Capturas (`toHaveScreenshot`):**

- **Pantallas:** `Ahora` (normal, térmica `below_base`, desconectado), onboarding 1 y 5, `Análisis` con huecos y eventos, `Informe` con potencial, prueba guiada en carga sostenida, `Ajustes` plegado, `Sesiones` vacía y con datos, `CoverageMatrix`, diálogo de exportación, `BottomBar` compacta.
- **Variantes:** claro y oscuro en expandido para todas; compacto para `Ahora` y `Ajustes`; inglés en `Ahora`, `Ajustes` e `Informe` (expansión de texto); vidrio `off` por defecto y `full` solo en `Ahora`.
- **Determinismo:** datos de escenario fijos, `page.clock` fijo, idioma por preferencia simulada, `reduced-motion`, animaciones desactivadas, fuentes del sistema del runner Windows, sin contenido remoto e identificadores fijos. Se enmascaran solo regiones legítimamente dinámicas (ninguna prevista).
- **Líneas base:** se generan **solo en el runner Windows de CI**, nunca en equipos locales. Se actualizan mediante un workflow explícito asociado a la PR del cambio, tras revisar el diff visual; jamás de forma automática porque fallen.
- Las capturas no sustituyen a los tests funcionales.

### 21. Property-based testing

Solo en Rust, con `proptest`, semillas reproducibles y los casos mínimos guardados en `proptest-regressions/` (confirmados en el repositorio):

| Propiedad | Módulo |
|---|---|
| Misma traza y reglas → mismo diagnóstico (NFR-009) | `diagnostics` |
| El rango redondeado hacia fuera contiene el rango sin redondear, está en múltiplos de 5 y nunca tiene decimales | `potential` |
| La agregación conserva mínimo y máximo globales, nunca une segmentos a través de un hueco y conserva los bordes de eventos | `telemetry` |
| Ningún identificador sembrado sobrevive a la anonimización, sea cual sea su posición | `export` |
| Serializar → deserializar es la identidad para todos los DTO de contrato | `ipc`, `export` |
| El parser NDJSON nunca entra en pánico ante bytes arbitrarios (*fuzzing* ligero) | `ipc` |
| Los niveles B y C nunca producen `mixed_limit` ni superan su techo de confianza | `classifier` |

No se aplica a componentes visuales. En TypeScript, la resolución de idioma y los esquemas se cubren con tablas de casos; solo se evaluará fast-check si aparece lógica TS con un espacio de entradas amplio.

### 22. Mutation testing

- **Propósito:** medir si los tests detectan errores reales en código crítico. No forma parte del bucle de desarrollo.
- **Herramientas candidatas** (no están en la constitución; cada una requiere piloto y registro en la tabla de versiones):
  - `cargo-mutants` para Rust (compatible con `nextest`; permite mutar solo el diff);
  - StrykerJS con el runner de Vitest para TS: compatibilidad nominal, a confirmar con Vitest en jsdom; no se fuerza con Browser Mode;
  - Stryker.NET para C#.
- **Piloto (T-MUT-003):** `diagnostics/potential.rs` y `diagnostics/classifier.rs` con `cargo-mutants`. Se registran mutantes, eliminados, supervivientes, sin cobertura, *timeouts*, errores y tiempo total.
- **Ampliación por orden de valor:** anonimización y exportación → agregación → reglas de alertas → paradas de la prueba guiada → normalización C# de temperatura → resolución de idioma y esquemas Zod del puente.
- **Excluido de inicio:** interfaz visual, CSS, código generado, configuración, envoltorios triviales, registros, scripts de build, copia del sistema de diseño y código de terceros.
- **Cuándo:** tras estabilizar un lote crítico (L10, L11, L13, L14, L18), en PR solo sobre el diff de módulos críticos y si cabe en el presupuesto, semanalmente completo en la ejecución programada y antes de cada release.
- **Umbral:** ninguno fijo al inicio. Se crea una línea base tras el piloto y después se prohíbe empeorarla en los módulos críticos. Cada superviviente se clasifica: cobertura ausente, aserción insuficiente, caso límite ausente, mutación equivalente, código muerto o comportamiento no observable.

### 23. Cobertura

- **Umbrales constitucionales, no arbitrarios** (principio XIII): ≥ 80 % de líneas y ≥ 70 % de ramas por componente; ≥ 95 % y ≥ 90 % en módulos críticos; en Rust, ramas como regiones LLVM; no bajar más de 0,5 puntos respecto a `main`.
- **Línea base:** se mide al cerrar L00/L01. Hasta que un componente tenga código suficiente, CI publica la cifra sin bloquear, y se bloquea desde el cierre de CHK-L01 (excepción E1 de `plan.md`, fecha límite 2026-10-31).
- **Herramientas:** `cargo llvm-cov nextest`, `@vitest/coverage-v8`, coverlet. Informes por componente en la PR.
- Se revisan especialmente las ramas de validación, errores, límites, reglas y paradas.
- No se escriben tests sin valor para subir la cifra. Una cobertura alta no sustituye a la aceptación por corpus, a los E2E ni al mutation testing.

### 24. Determinismo: tiempo, aleatoriedad y entorno

| Fuente de no determinismo | Solución |
|---|---|
| Reloj de pared y monotónico (Rust) | Puerto `Clock` inyectado; `FakeClock` con avance manual. Se prueban saltos del reloj del sistema, cambio de hora (sesiones de 24 h y formato de registros) y suspensión |
| Temporizadores TS | `vi.useFakeTimers()` en Vitest; `page.clock` en Playwright |
| .NET | `TimeProvider` inyectado; fake propio o `Microsoft.Extensions.TimeProvider.Testing`, que solo se añade tras evaluarlo (§ 5) |
| UUID y nonce | Puertos `IdGenerator` y `NonceSource` con secuencias conocidas |
| Zona horaria e idioma | Los formateadores reciben idioma y zona explícitos; los tests de `Intl` fijan `timeZone`; en E2E el idioma se fija por preferencia simulada, no por el sistema del runner |
| Variables de entorno | Solo `TW_DEV_*` y `PUBLIC_*` validadas; los tests no leen el entorno real del desarrollador |
| Hardware | Trazas replay; nunca sensores reales en CI |
| Concurrencia | Supervisor y escritor por lotes probados con canales y puntos de sincronización explícitos, no con esperas |

### 25. Dependencias externas y hardware

| Dependencia | Sustituto en pruebas rápidas | Prueba real |
|---|---|---|
| CPU y sensores (LibreHardwareMonitorLib, MSR, SMU) | Colector replay; `FakeHardware`/`FakeSensor` en C#; proveedor de registros falso | Corpus y matriz de hardware (T019b, T109); manual en release |
| Contadores PDH, energía, idioma, tema, transparencia y movimiento de Windows | Puertos Rust con fakes | Suite `windows-smoke` en el runner (PDH existe sin sensores) |
| GitHub Releases | Servidor HTTP local + clave de prueba | Un ensayo de release en un repositorio de pruebas (T102) |
| WebView2 | jsdom (componentes) / Chromium (`frontend`) | Proyecto `app` |
| Diálogos, bandeja, notificaciones, autostart, varios monitores | Puertos con fakes | Lista manual de release |
| Controlador de bajo nivel y UAC | Estado simulado `installable/denied/error` | Hardware real con evidencias (T019a, T020) |
| Carga de CPU del diagnóstico guiado | Generador falso que publica `throughput_ops_s` de la traza | Solo en hardware real, voluntaria y manual; **nunca en CI** |

Para cada dependencia se prueba: éxito, error, *timeout*, respuesta inválida, respuesta parcial, servicio caído y reintento cuando aplique.

### 26. Rendimiento y seguridad verificable

- **Rendimiento:**
  - benchmark de `AnalysisChart` (4 pistas × 3.000 puntos) en el harness y en hardware objetivo (T060);
  - consumo pasivo y memoria durante 1 h (NFR-003, SC-006), latencia de muestra (NFR-002), arranque (NFR-010) y crecimiento del almacenamiento (NFR-016) medidos en el equipo de referencia antes de cada release (T107, T108).
  - En CI solo se registran tendencias que **avisan**, porque las VM no son representativas.
- **Seguridad:**
  - tráfico cero (gate 12);
  - CSP sin orígenes remotos;
  - lista de comandos y capacidades igual al contrato;
  - rechazo de nonce, secuencia y tamaño;
  - *fuzzing* ligero del parser;
  - sin rutas desde la WebView;
  - firma inválida rechazada;
  - redacción de registros;
  - almacenamiento web vacío.

### 27. Modelo escalonado de ejecución y presupuesto de coste

| Nivel | Cuándo | Qué se ejecuta | Coste orientativo (medir en L00) |
|---|---|---|---|
| 0 · Edición | Tras cada cambio pequeño | Diagnósticos del editor, `pnpm check` o `cargo check` del paquete afectado, test concreto en diagnóstico | Segundos |
| 1 · Ultrarrápida | Mientras se implementa | `vitest related <ficheros> --run`, `cargo nextest run -E '<filtro>'` del módulo, `dotnet test --filter` de la clase | Segundos |
| 2 · Lote (checkpoint) | Al cerrar el lote | Nivel 0 completo de las pilas tocadas + unitarias, componentes, integración y aceptación afectadas; corpus si toca el motor | Segundos a pocos minutos |
| 3 · Historia / PR | Al abrir o actualizar una PR | Todo lo anterior completo por pila + cobertura + build + E2E `@smoke` y afectados de ambos proyectos + accesibilidad y visual si hay cambios de interfaz | Minutos |
| 4 · Completa | `main`, programada, release | Todo: E2E completos, matriz de entorno, visual, accesibilidad, mutation programado, rendimiento y verificación del artefacto | Alto |

No se ejecuta el nivel 4 tras cada cambio. Si una suite se vuelve lenta: se identifican los tests lentos (informes de `nextest` y Vitest), se divide, se mejora el aislamiento, se paraleliza, se reduce infraestructura o se mueve a un checkpoint superior. No se eliminan tests útiles por lentos.

### 28. Selección de tests afectados

| Cambio en | Nivel 1–2 mínimo | Amplía a PR completa si… |
|---|---|---|
| `src-tauri/src/diagnostics/**`, `ruleset-*.json` | Unitarias del módulo + propiedades + **corpus completo** + test «ruleset coincide con spec» | Cambia una regla o un umbral |
| `src-tauri/src/ipc/**`, `packages/contracts/**` | Contrato en las **tres** pilas + conformidad Zod + integración IPC | Siempre (contrato compartido) |
| `src-tauri/src/storage/**`, migraciones | Integración de almacenamiento + migraciones desde todas las versiones | Siempre |
| `src-tauri/src/export/**` | Exportación e importación + fuga de identificadores + propiedades | — |
| Comandos Tauri, `capabilities/`, `tauri.conf.json` | Integración de comandos + test de superficie | Siempre + E2E `app` completo |
| `apps/sensor-agent/**` | `dotnet test` de la clase y del proyecto + contrato | Protocolo o normalización de temperatura |
| `apps/desktop/src/features/<x>/**` | `vitest related` + componentes de la feature + E2E `frontend` con etiqueta de la feature | La feature tiene E2E `app` |
| `apps/desktop/src/lib/bridge/**` | Unitarias del puente + conformidad de fixtures | E2E `frontend` completo |
| `design/**`, copia del diseño, `tokens.css` | `check` y `build` del harness + igualdad de copia + contrato de estilos | Visual y accesibilidad completos |
| Catálogos `es`/`en` | Igualdad de claves y literales + componentes afectados | Visual de expansión de texto |
| `vite.config.ts`, `tsconfig*`, `svelte.config.js`, `package.json`, `Cargo.toml`, `*.csproj`, archivos de bloqueo | Nivel 0 completo | Siempre nivel 3 completo |

### 29. Estructura de ficheros

Se respeta la estructura de `plan.md`. Orientación:

```text
apps/desktop/
  src/**/x.test.ts                 unitarias TS (proyecto Vitest "unit", entorno node)
  src/**/x.svelte.test.ts          componentes (proyecto Vitest "component", jsdom)
  src/test-support/                builders, FakeBridge, escenarios de mock
  e2e/
    frontend/*.spec.ts             proyecto Playwright "frontend"
    app/*.spec.ts                  proyecto Playwright "app"
    support/                       fixtures (errores, red, almacenamiento), perfiles sembrados, allowlist
    visual/                        specs visuales; líneas base generadas en CI
  src-tauri/
    src/**/mod.rs                  #[cfg(test)] unitarias junto al código
    src/test_support/              TraceBuilder, FakeClock, puertos falsos (solo cfg(test) o característica e2e)
    tests/integration_*.rs         integración
    tests/acceptance_*.rs          aceptación y corpus
    proptest-regressions/          casos mínimos guardados
apps/sensor-agent/Tests/{Unit,Integration,Protocol}/
packages/contracts/fixtures/{valid,invalid,app}/
packages/trace-fixtures/{synthetic,recorded,labeled}/ + tools/ (replay)
tests/hardware/                    protocolos manuales y evidencias de la matriz real
```

Sin carpetas vacías ni profundidad innecesaria; los nombres siguen la convención de la pila.

### 30. Comandos

Hoy **no existe ningún script** en la aplicación (solo `check` y `build` en `design/harness` y `design/mockup`, ejecutados con npm). Los siguientes son los nombres propuestos, alineados con `quickstart.md`; se crean en T006/T-TEST-006 y no se imponen si ya existiera otra convención válida:

| Propósito | Comando propuesto |
|---|---|
| Tipos y Svelte | `pnpm check` |
| Lint y formato | `pnpm lint`, `pnpm format:check`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `dotnet format --verify-no-changes` |
| Unitarias y componentes TS | `pnpm test`, `pnpm test:unit`, `pnpm test:component`, `pnpm vitest related <ficheros> --run`, `pnpm vitest <fichero> -t "<nombre>"` |
| Rust | `cargo nextest run --workspace`, `cargo nextest run -E 'test(/fr069/)'`, `cargo nextest run --test acceptance_corpus` |
| .NET | `dotnet test apps/sensor-agent --filter "Category=Unit"` (o `Integration`, `Protocol`) |
| Suite de lote | `pnpm verify:batch -- <lote>`, que encadena lo afectado de las tres pilas |
| Suite de PR | `pnpm verify:pr` |
| E2E | `pnpm test:e2e` (`frontend`), `pnpm test:e2e:app`, `pnpm test:e2e:smoke`, `pnpm test:e2e:ui` (modo UI), `pnpm test:e2e:headed`, `pnpm test:e2e:debug`, `pnpm test:e2e:visual`, `pnpm test:e2e:a11y`, `pnpm test:e2e:matrix` (sustituye a `cross-browser`) |
| Cobertura | `pnpm test:coverage`, `cargo llvm-cov nextest --workspace`, `dotnet test --collect:"XPlat Code Coverage"` |
| Mutation | `cargo mutants --in-diff <diff>` (tras el piloto), `pnpm test:mutation`, `dotnet stryker` |
| Sistema de diseño | `pnpm design:check` (harness `check` + `build` + igualdad de copia) |

### 31. CI

Runner `windows-2025` (constitución). Orden del principio XVI: instalación con archivos de bloqueo → formato y lint → `pnpm check` → pruebas → compilación → E2E. Un paso fallido detiene los siguientes. Cachés de pnpm, cargo (`target/` por clave de lockfile) y NuGet.

#### 31.1 Pull request

1. Nivel 0 de las tres pilas en paralelo, más trazabilidad, igualdad de la copia de diseño, catálogos y licencias.
2. Unitarias, componentes e integración por pila, con cobertura y umbrales.
3. Aceptación: corpus si cambian `diagnostics/`, reglas o contratos; siempre en PR a `main` que toque el backend.
4. Compilación `e2e` de la aplicación y build de Vite.
5. E2E `@smoke` de ambos proyectos + afectados por filtro de rutas (§ 28).
6. Si la PR toca interfaz o tokens: accesibilidad y visual de las pantallas afectadas en ambos temas e idiomas.
7. Mutation sobre el diff de módulos críticos si el plan del lote lo exige.

#### 31.2 Rama principal

E2E completos de `frontend` y `app`, visual y accesibilidad completos, cobertura publicada e informe de duración por suite.

#### 31.3 Programada (nocturna y semanal)

- **Nocturna:** matriz de entorno completa, `--repeat-each=3` sobre E2E para detectar inestabilidad, tendencias de rendimiento, auditoría de dependencias.
- **Semanal:** mutation testing completo de los módulos del alcance e informe histórico.

#### 31.4 Release

1. Construir el instalador NSIS firmado y el manifiesto de actualización firmado.
2. Ejecutar el nivel 4 sobre la compilación `e2e` **del mismo commit**.
3. **Verificar el artefacto real** en VM limpias (Windows 11 25H2 y 24H2, Windows 10 22H2):
   - instalación silenciosa por usuario y sin elevación;
   - arranque y comprobación nativa (proceso vivo, ventana creada, códigos `app_started` y `collector_connected` en el registro JSON, ningún `error`);
   - tráfico cero con el actualizador apagado;
   - firma Authenticode;
   - desinstalación conservando y eliminando datos.
4. Lista manual firmada: bandeja, notificaciones, diálogos nativos, varios monitores, Narrador y NVDA, controlador de bajo nivel en hardware real y matriz de hardware (T109).

### 32. Tests inestables

- Un test inestable es un defecto. Se investiga estado compartido, condiciones de carrera, esperas, datos compartidos, orden, temporizadores, animaciones, arranque del proceso `app`, puertos y paralelismo.
- **Reintentos:** 0 en local; 1 en CI solo para Playwright y `nextest`, y un test que pasa al reintentar **se marca como flaky** en el informe y abre incidencia automáticamente.
- Cuarentena como máximo 5 días laborables con incidencia (principio XIII); un test crítico (P1) no se pone en cuarentena sin aprobación explícita.
- No se «arregla» subiendo reintentos, *timeouts* ni añadiendo `waitForTimeout`.

### 33. Optimización

- La lógica se prueba en Rust nativo y en memoria; no se levanta la aplicación ni un navegador para lógica aislable.
- Se ejecutan primero los tests cercanos y después la suite del lote; los E2E `app` quedan para PR y superiores.
- Paralelismo por pila y *sharding* de Playwright solo si la suite supera el presupuesto medido.
- La matriz de entorno, los visuales completos y el mutation testing son programados o de release.
- No se repite la misma combinación en varias capas: si el corpus demuestra una clase, el componente solo verifica cómo se presenta.

### 34. Integración con Spec Kit

- **`spec.md`:** cada historia mantiene criterios verificables con errores, límites, estados y comportamiento observable. Los parámetros viven en «Parámetros iniciales». `/speckit-clarify` resuelve criterios que no puedan traducirse a una aserción.
- **`plan.md`:** su «Estrategia de pruebas» se actualiza para recoger esta sección: proyectos Vitest y Playwright, compilación `e2e`, puertos sustituibles, perfiles sembrados, suites y checkpoints, CI escalonada, piloto de mutation y presupuesto de tiempos. La comprobación de la constitución cubre los principios XIII, XIV, XVI y XVII.
- **`tasks.md`:**
  - las tareas se agrupan en los lotes del § 7;
  - cada lote lleva su plan (§ 8) y termina con una tarea de checkpoint `CHK-Lxx` que lista la suite de cierre;
  - las tareas de test van **dentro** del lote y no se alternan «implementar A / test A / implementar B / test B»;
  - las tareas de test existentes (T012, T013, T027, T035, T037a–c, T046, T052, T059, T066, T072, T087, T094, T103) se conservan y se ubican en su lote.
- **`/speckit-analyze`:** verifica que cada requisito tiene fila en `traceability.md` y cada lote tiene checkpoint.
- **`/speckit-implement`:** procesa un lote cada vez, ejecuta solo la suite del lote al cerrarlo, registra el resultado (orden, código de salida, errores y avisos; principio XVI) y marca las tareas.

### 35. Definition of Checkpoint (cierre de lote)

Un lote se cierra cuando:

- la implementación prevista está completa y compila;
- el nivel 0 de las pilas tocadas está a cero errores y avisos;
- existen los tests obligatorios del plan del lote y pasan las suites afectadas;
- la aceptación relacionada pasa, incluido el corpus si el lote toca el motor;
- existen los *fixtures*, *builders* y fakes que necesitan los lotes siguientes;
- no hay errores críticos ni tests pendientes o ignorados sin justificación;
- `traceability.md` está actualizado;
- no queda deuda temporal del lote.

No es obligatorio en cada checkpoint: todos los E2E, toda la matriz, todas las capturas, el mutation testing completo ni las suites no afectadas.

### 36. Definition of Done

**Historia terminada** cuando:

- todos sus lotes están cerrados;
- cada criterio de aceptación tiene test trazable o verificación manual justificada;
- están cubiertos reglas, errores, límites y estados principales;
- las dependencias externas están controladas y los tests son deterministas, sin esperas arbitrarias ni tests ignorados;
- pasa la suite de PR, incluidos los E2E críticos, en los tamaños y la matriz requeridos;
- las capturas coinciden o se actualizaron intencionadamente;
- no hay errores inesperados de consola o página;
- la cobertura cumple la constitución sin deterioro;
- no queda deuda de tests;
- los comandos están documentados y los tests integrados en CI.

Mutation testing es obligatorio solo si lo fija el plan del lote, afecta a código crítico, su coste es proporcional y la herramienta ya ha superado el piloto.

**Adicional cuando la historia usa Playwright:**

- pasan los smoke y E2E obligatorios;
- no hay excepciones, `console.error` ni peticiones a otros orígenes no declaradas;
- los flujos funcionan en los tamaños mínimo e inicial;
- la matriz de entorno requerida está verificada;
- las capturas son correctas o están actualizadas con revisión;
- los perfiles, escenarios de mock y trazas están documentados;
- no hay `waitForTimeout`;
- cada test funciona de forma independiente;
- los artefactos de diagnóstico están configurados.

### 37. Tareas para `tasks.md`

Se añaden **solo** las que aportan trabajo real. Entre paréntesis, relación con tareas existentes.

- **General:**
  - T-TEST-001 Inventariar versiones reales y confirmar el runtime de pruebas de cada pila (T006);
  - T-TEST-002 Crear puertos sustituibles (reloj, IDs, nonce, almacenamiento, diálogos, ventana, bandeja, notificaciones, autostart, HTTP y energía/idioma/tema de Windows) y `test_support`;
  - T-TEST-003 Definir suites, etiquetas y checkpoints `CHK-Lxx` en `tasks.md`;
  - T-TEST-004 Crear los scripts del § 30 y documentarlos en `quickstart.md`;
  - T-TEST-005 Medir tiempos iniciales por suite y fijar presupuestos;
  - T-TEST-006 Implementar la selección de tests afectados por rutas en CI (§ 28).
- **Unitarias:**
  - T-UNIT-001 `TraceBuilder` y *builders* de snapshot, cobertura e informe;
  - T-UNIT-002 `FakeClock`, `FakeIdGenerator` y fakes C# de hardware.
  - Las reglas del motor ya están en T037a–c, T046 y T052.
- **Componentes:**
  - T-COMP-001 Configurar Vitest con los proyectos `unit` y `component` para Svelte 5 y jsdom;
  - T-COMP-002 `FakeBridge` validado con Zod y escenarios;
  - T-COMP-003 Utilidades de consulta por catálogo (`t('key')`);
  - T-COMP-004 Tests de componentes con comportamiento en `design/harness`: foco de `Dialog`, teclado de `SegmentedControl` y del gráfico, orden de `CpuAdvancedTable` y máquina de estados de `OnboardingFlow`.
- **Integración y aceptación:**
  - T-INT-001 Pipeline IPC con colector replay;
  - T-INT-002 Almacenamiento, migraciones y fallos inyectados (T104);
  - T-INT-003 Superficie de comandos frente al contrato;
  - T-INT-004 Servidor local de releases y clave de prueba (T103);
  - T-ACC-001 `traceability.md` y check de CI (T009a);
  - T-ACC-002 Aceptación por corpus con métricas SC (T045).
- **Playwright:**
  - T-PLAY-001 Configurar los proyectos `frontend` y `app`;
  - T-PLAY-002 Spike de conducción de WebView2 por CDP en Tauri y de la compilación `e2e`;
  - T-PLAY-003 Smoke;
  - T-PLAY-004 *Fixture* de errores, red y almacenamiento con allowlist vacía;
  - T-PLAY-005 Perfiles sembrados y aislamiento por directorio temporal;
  - T-PLAY-006 E2E-01 a E2E-13 distribuidos en sus lotes;
  - T-PLAY-007 Tamaños y escala 200 %;
  - T-PLAY-008 Matriz de entorno;
  - T-PLAY-009 Contrato de estilos y líneas base visuales en CI con workflow de actualización revisada;
  - T-PLAY-010 Artefactos de fallo.
- **Calidad:**
  - T-A11Y-001 axe por pantalla y estado, emulación de temas de contraste y movimiento reducido, y guion manual de Narrador y NVDA (T106);
  - T-QUAL-001 Cobertura por pila con umbrales constitucionales y regla de no deterioro;
  - T-QUAL-002 Detección de tests inestables (informe y apertura de incidencia);
  - T-QUAL-003 Verificación nativa del artefacto de release en VM limpias;
  - T-QUAL-004 Tendencias de rendimiento (T060, T107, T108);
  - T-QUAL-005 Estudio moderado con personas para SC-001.
- **Mutation:**
  - T-MUT-001 Piloto de `cargo-mutants` sobre `potential.rs` y `classifier.rs`;
  - T-MUT-002 Línea base y revisión de supervivientes;
  - T-MUT-003 Evaluar StrykerJS (Vitest) y Stryker.NET;
  - T-MUT-004 Ejecución selectiva en PR y programada; registrar las herramientas en la constitución.

### Preguntas abiertas sobre testing

Ninguna bloquea el resto del trabajo. Cada una lleva una suposición conservadora provisional y una tarea para confirmarla.

1. **¿Se puede conducir WebView2 con Playwright por CDP en Tauri 2 en el runner `windows-2025`?** Suposición: sí, con la compilación `e2e`. Si falla, se evalúa WebDriver (`tauri-driver` + `msedgedriver`), que no está en el stack. → T-PLAY-002.
2. **¿Basta `mockIPC` de `@tauri-apps/api/mocks` para el proyecto `frontend`, incluidos los eventos?** Suposición: sí; si no, un `FakeBridge` propio inyectado en la build de test. → T-COMP-002.
3. **¿Es estable el renderizado del vidrio (`backdrop-filter`) en el runner sin GPU?** Suposición: no lo suficiente; las líneas base usan vidrio `off` salvo una captura con tolerancia. → T-PLAY-009.
4. ~~Contradicción de NFR-008 (Windows 10 1809+) con la constitución.~~ **Resuelta (2026-09-18):** NFR-008 corregido a Windows 11 24H2/25H2 y Windows 10 22H2, x64.
5. ~~Escala 200 % frente a ventana mínima en 1080p.~~ **Resuelta (2026-09-18, constitución 1.5.1):** cuando la altura útil del monitor no alcanza 600 px lógicos, la ventana mínima pasa a 480×500; se prueban 480×600 y 480×500 a 200 %.
6. ~~Equipos reales disponibles.~~ **Resuelta (2026-09-18):** equipo de desarrollo AMD Ryzen 5 2600X (Zen+, sobremesa, sin batería, Windows 11 25H2; «AMD anterior») más Intel híbrido (12.ª gen o posterior), Intel anterior (≤ 11.ª gen), un portátil y AMD Zen 3/4/5. Cubren la matriz de T019b y T109; queda por asignar quién ejecuta cada verificación manual.
7. **¿Se autoriza el piloto de `cargo-mutants`, StrykerJS y Stryker.NET** aunque no estén en la constitución? Suposición: sí como piloto local; entran en CI solo tras registrarlos (enmienda PATCH). → T-MUT-001/003.
8. **¿Qué duración máxima debe tener el pipeline de una PR?** Suposición: ≤ 15 minutos de mediana; se mide en T-TEST-005 y se ajusta el reparto entre PR y ejecución programada.
9. **¿Se pueden añadir Vitest y Testing Library al harness de `design/`** para probar el comportamiento de sus componentes? Suposición: sí, como dependencias de desarrollo del harness con las mismas versiones que la aplicación. → T-COMP-004.
10. **¿Qué historias se consideran de alto riesgo a efectos de mutation testing obligatorio?** Suposición: HU-02, HU-03, HU-04, HU-07, HU-14 y HU-15.
