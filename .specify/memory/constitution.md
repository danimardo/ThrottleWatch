# Constitución de ThrottleWatch

Esta constitución es el marco innegociable del proyecto. Toda especificación, plan, tarea,
revisión e implementación DEBE cumplirla; ninguna fase posterior puede contradecirla sin una
enmienda previa aprobada según la sección «Gobernanza». Las palabras DEBE, NO DEBE y PUEDE
tienen valor normativo; «DEBERÍA» indica una recomendación cuya omisión exige justificación
escrita en `plan.md`.

## Principios fundamentales

### I. Evidencia antes que afirmación

La aplicación DEBE distinguir entre una observación, una inferencia y una confirmación directa. No DEBE equiparar temperatura elevada con pérdida de rendimiento ni tiempo en estado de throttling con porcentaje perdido. Toda conclusión visible DEBE incluir las señales que la sustentan, el intervalo analizado y un nivel de confianza. Si faltan datos, DEBE decirlo explícitamente.

**Motivo:** el valor del producto depende de evitar falsos diagnósticos y recomendaciones de refrigeración innecesarias.

### II. Hardware heterogéneo y degradación elegante

Ningún sensor concreto se considerará universal. El sistema DEBE descubrir capacidades en tiempo de ejecución, normalizar nombres y unidades y trabajar con valores ausentes. Una CPU no reconocida, un sensor sin valor o la falta del controlador de bajo nivel NO DEBEN bloquear la aplicación completa. La interfaz DEBE mostrar qué puede y qué no puede medir en el equipo actual.

**Motivo:** Intel, AMD, generaciones antiguas, portátiles OEM y placas base exponen datos diferentes.

### III. Causalidad prudente y factores de confusión

El motor DEBE considerar temperatura, carga, frecuencia activa y frecuencia base, potencia, límites de potencia/corriente y sus cambios, plan energético, alimentación y tiempo, incluida la ventana de turbo tras cada inicio de carga. Antes de recomendar mejor refrigeración, DEBE evaluar causas alternativas, incluida la gestión térmica del fabricante. Toda cifra de rendimiento DEBE proceder de una medición directa comparable (misma carga, mismo equipo, mismo contexto) o de un modelo físico explícito y versionado con sus entradas visibles, y DEBE expresarse como rango o tramo cuando la incertidumbre sea material. Una comparación de relojes con una referencia aprendida no es una estimación de rendimiento válida.

**Motivo:** una frecuencia baja puede deberse a reposo, PL1/PPT, modo silencioso, batería o firmware, no al calor.

### IV. Privilegio mínimo y aislamiento

La interfaz Tauri DEBE ejecutarse sin elevación. El acceso de bajo nivel DEBE limitarse al componente auxiliar estrictamente necesario, con protocolo local autenticado, comandos permitidos explícitamente y sin aceptar rutas o código arbitrario. La instalación o reparación del controlador DEBE requerir una acción consciente del usuario.

**Motivo:** leer sensores no justifica elevar toda la superficie de interfaz.

### V. Local primero y privacidad por defecto

El muestreo, diagnóstico, almacenamiento y exportación DEBEN funcionar sin Internet. No habrá telemetría remota por defecto. Los archivos exportados DEBEN permitir excluir identificadores de hardware. La política de retención DEBE ser visible y configurable.

Además:

- NO DEBE existir telemetría, analítica de uso ni envío de informes de fallo, tampoco anónimos ni opcionales.
- La única comunicación de red permitida es la del actualizador cuando el usuario lo activa expresamente, contra un endpoint fijo de GitHub Releases y sin identificadores. Con el actualizador apagado, el tráfico de red de la aplicación DEBE ser cero.
- Abrir documentación en línea solo ocurre a petición del usuario y mediante el navegador del sistema, nunca dentro de la WebView.

**Motivo:** los datos de hardware y hábitos de uso pertenecen al usuario.

### VI. Visualización fiel, comprensible y accesible

Los gráficos DEBEN representar unidades, escalas, ausencia de datos e incertidumbre sin engaño. El color nunca será el único canal de estado. El resumen debe ser comprensible para una persona no experta y el detalle debe satisfacer a un usuario técnico mediante divulgación progresiva. Animaciones y efectos decorativos NO DEBEN dificultar la lectura ni degradar el rendimiento.

**Motivo:** una apariencia vistosa solo aporta valor si mejora la comprensión.

### VII. Reproducibilidad y pruebas con trazas

El motor de diagnóstico DEBE ser determinista para una configuración y una traza dadas. Cada regla DEBE tener pruebas unitarias; los escenarios completos DEBEN poder reproducirse con trazas sintéticas y grabadas, sin depender de ejecutar la prueba térmica en CI. Los cambios de umbrales DEBEN quedar versionados.

Todo número que la aplicación use para decidir (umbrales, ventanas, duraciones, límites de seguridad, cadencias de avisos) DEBE estar escrito en `spec.md` como parámetro con identificador y versión, y cargarse desde configuración versionada; NO DEBE quedar como literal disperso en el código ni a criterio de quien programa. Cada informe congelado DEBE registrar la versión de reglas y parámetros con la que se generó.

**Motivo:** el hardware real es difícil de reproducir y los errores de clasificación son costosos.

### VIII. Seguridad térmica y control del usuario

Cualquier carga guiada DEBE ser voluntaria, explicar qué hará, permitir cancelación inmediata y detenerse ante los límites de seguridad configurados, pérdida del sensor crítico o fallo del auxiliar. El programa NO DEBE modificar voltajes, límites de potencia, curvas de ventilador, BIOS ni frecuencias. La única escritura permitida en registros del procesador es la limpieza de bits de registro de estado (por ejemplo, los de razones de limitación), que no altera el funcionamiento; DEBE estar limitada a una lista cerrada de registros y bits en el componente privilegiado.

**Motivo:** la herramienta diagnostica; no toma control del hardware.

### IX. Separación estricta de capas

La lógica de negocio y la de presentación DEBEN vivir en capas distintas con dependencias en un solo sentido: **interfaz → puente de comandos → servicios de aplicación → dominio**. En concreto:

- **Dominio (Rust):** el motor de diagnóstico, la normalización de dominio, la clasificación de sesiones, el cálculo de potencial, la anonimización y las reglas de avisos son funciones puras sin E/S. NO DEBEN depender de Tauri, SQLite, Win32, reloj del sistema ni sistema de archivos; reciben datos y configuración, y devuelven resultados.
- **Servicios de aplicación (Rust):** orquestan sidecar, persistencia, ciclo de vida, bandeja, notificaciones y actualizador. Los comandos Tauri son adaptadores delgados que validan parámetros tipados y delegan; NO DEBEN contener reglas de negocio.
- **Sidecar (.NET):** solo adquiere, identifica y normaliza sensores y topología. NO DEBE diagnosticar, persistir, acceder a red ni conocer la interfaz.
- **Interfaz (Svelte):** solo presenta y gestiona estado de presentación (navegación, foco, selección, formularios). NO DEBE calcular diagnósticos, interpretar nombres de sensores, acceder a disco o red ni decidir umbrales. Toda llamada a Tauri DEBE pasar por un único módulo puente de adaptadores; ningún componente de pantalla importa `@tauri-apps/api` directamente.
- **Contratos:** los mensajes entre capas y procesos se definen una sola vez en JSON Schema (`packages/contracts/`), y los tipos Rust, TypeScript y C# DEBEN verificarse contra esos esquemas en CI. Los esquemas Zod del puente (principio XIV) DEBEN aceptar y rechazar exactamente los mismos fixtures válidos e inválidos del corpus de contratos que el JSON Schema canónico.

**Motivo:** un motor puro es determinista y testeable sin hardware; una interfaz sin lógica puede rediseñarse sin riesgo de cambiar conclusiones.

### X. Datos locales, estructurados y gobernados por el usuario

- Todos los datos persistentes (muestras, eventos, sesiones, informes, referencias, preferencias, estado de onboarding, geometría y estado del actualizador) DEBEN almacenarse en **una única base SQLite** en `%LOCALAPPDATA%\ThrottleWatch\`, en modo WAL, con claves foráneas activas y esquema versionado mediante migraciones numeradas solo hacia delante.
- Solo el backend Rust DEBE escribir en la base. La WebView NO DEBE usar `localStorage`, `sessionStorage`, IndexedDB ni cookies para estado duradero; el sidecar NO DEBE persistir nada.
- El registro de Windows solo PUEDE usarse para lo que el sistema operativo exige (entrada de inicio con Windows y la del desinstalador). No se admiten ficheros de configuración sueltos.
- Tiempos en UTC (milisegundos desde época en la base; ISO 8601 en exportaciones). Cada magnitud tiene una unidad canónica fija: °C, MHz, W, %. Un valor ausente es `NULL` con su indicador de calidad; NUNCA cero ni interpolación.
- Registros técnicos en `%LOCALAPPDATA%\ThrottleWatch\logs\`, JSON por líneas, rotación de 5 ficheros de 5 MB, sin identificadores personales ni del equipo, según el principio XVII.
- Exportaciones en CSV (UTF-8, separador coma, punto decimal, claves técnicas en inglés) e informe JSON validado por JSON Schema versionado, escritos solo en la carpeta que el usuario elija.
- La retención y el espacio ocupado DEBEN ser visibles. Borrar datos y restablecer son transacciones distintas; una base dañada se aparta con fecha y NUNCA se borra sin avisar.
- Los datos no se cifran en reposo: se protegen con los permisos del perfil de Windows del usuario y no contienen credenciales. Cualquier dato que exigiera cifrado requiere enmienda previa.

**Motivo:** una sola fuente transaccional hace fiables la retención, el borrado, la exportación y la recuperación.

### XI. Usabilidad para personas no expertas

- **Respuesta primero:** cada pantalla empieza por la conclusión en lenguaje llano; las evidencias y los datos técnicos aparecen por divulgación progresiva. Nada «avanzado» se muestra desplegado la primera vez.
- **Vocabulario controlado:** un mismo concepto se nombra igual en toda la aplicación y en ambos idiomas, según un glosario único. Cada término técnico (throttling, TjMax, frecuencia base, PROCHOT) se explica brevemente donde aparece por primera vez.
- **Valores iniciales prudentes:** de fábrica, la aplicación no usa la red, no pide elevación, no arranca con Windows, no notifica y no aplica carga. Todo lo que cambia eso exige un gesto explícito y reversible.
- **Consecuencias visibles:** una acción destructiva o irreversible DEBE confirmarse describiendo su alcance exacto; un control deshabilitado DEBE explicar por qué.
- **Estados completos:** cada flujo DEBE resolver sus estados de carga, vacío, datos degradados y error; ningún fallo produce una pantalla en blanco. Un mensaje de error dice qué ha pasado, qué sigue funcionando y qué puede hacer el usuario.
- **Comportamiento de escritorio Windows:** la aplicación se comporta como un programa nativo, no como una página web: barra de título propia, sin subrayados al pasar el ratón o enfocar, sin cursor de mano en botones (reservado a los puntos interactivos del gráfico), atajos de teclado documentados e instancia única.
- **Coherencia:** el sistema de diseño aprobado (`design/`) es la única fuente de tokens, componentes y patrones (véase «Estándares de ingeniería»).

**Motivo:** el público principal no tiene conocimientos de hardware; la confianza se gana con claridad, no con cantidad de datos.

### XII. Accesibilidad verificable

La aplicación DEBE cumplir **WCAG 2.2 nivel AA**, interpretado para software de escritorio según WCAG2ICT, en ambos temas, ambos idiomas y todos los niveles de vidrio. Como mínimo:

- **Teclado:** toda función es operable solo con teclado, en orden lógico y sin trampas de foco; el foco es siempre visible (contraste ≥ 3:1) y nunca queda oculto bajo barras fijas.
- **Contraste:** texto ≥ 4,5:1; texto grande, iconos de estado, bordes de controles y trazos de gráficos ≥ 3:1.
- **Canales redundantes:** ningún estado se comunica solo por color; se combina texto, icono y color.
- **Lectores de pantalla:** cada control tiene rol, nombre y estado expuestos a UI Automation a través de WebView2; los cambios de conclusión y los avisos se anuncian mediante regiones vivas (`polite`; `assertive` solo para paradas de seguridad y pérdida del colector). El atributo `lang` refleja el idioma efectivo.
- **Gráficos:** navegación por teclado, resumen textual y tabla equivalente del rango visible.
- **Escala y tamaño:** la interfaz es plenamente usable del 100 % al 200 % de escala de Windows y a 480×600 (o a 480×500 cuando la altura útil del monitor no alcanza 600 px lógicos; por debajo de 500, ventana maximizada al área útil con desplazamiento vertical y barra de título, banner global y `Detener ahora` siempre visibles) sin ocultar acciones críticas; los objetivos táctiles o de puntero miden al menos 24×24 px.
- **Movimiento y transparencia:** se respetan «Reducir movimiento» y «Efectos de transparencia» de Windows; con movimiento reducido todo es instantáneo; ningún contenido parpadea más de tres veces por segundo.
- **Temas de contraste de Windows:** con `forced-colors: active` todo el contenido y el foco siguen siendo legibles y operables.
- **Tiempo:** ninguna acción exige responder en un tiempo límite.

**Motivo:** una herramienta de diagnóstico para cualquier usuario de Windows debe poder usarla cualquier persona, sea cual sea su forma de interactuar.

### XIII. Testabilidad y cobertura mínima

- El código DEBE diseñarse para probarse sin hardware real: dependencias externas (sidecar, reloj, sistema de archivos, Win32, red) detrás de interfaces sustituibles; el motor se prueba con trazas.
- Las reglas del motor de diagnóstico y las paradas de seguridad se desarrollan con pruebas primero: la prueba DEBE existir y fallar antes de la implementación.
- Toda corrección de un defecto DEBE incluir una prueba de regresión que falle sin la corrección.
- **Cobertura mínima bloqueante en CI**, medida por componente (Rust, TypeScript y C# por separado):
  - General: ≥ 80 % de líneas y ≥ 70 % de ramas.
  - Módulos críticos: ≥ 95 % de líneas y ≥ 90 % de ramas. Son críticos el motor de diagnóstico y la clasificación de sesiones, la normalización de sensores (sidecar) y la validación de contratos (Rust), la anonimización y exportación, la parada de seguridad y watchdog de la prueba guiada, la máquina de estados del actualizador, las migraciones de SQLite, la resolución de idioma, los esquemas de validación de frontera (principio XIV), los módulos de configuración (principio XV) y la redacción de datos sensibles en registros (principio XVII).
  - En Rust, «ramas» se mide como regiones de LLVM, porque la cobertura de ramas de `cargo-llvm-cov` requiere compilador nightly.
  - La cobertura de un componente NO DEBE bajar más de 0,5 puntos respecto a `main`.
  - Solo se excluyen de la medición el código generado, los puntos de entrada `main` sin lógica, la copia sincronizada de `design/` (verificada en su propio harness) y los fixtures. Cualquier otra exclusión DEBE justificarse en la PR que la introduce.
- Una prueba inestable se corrige o se pone en cuarentena con incidencia abierta en un máximo de 5 días laborables; NO DEBE desactivarse sin rastro.

**Motivo:** el hardware real no se puede reproducir en CI; solo un diseño testable y un umbral objetivo mantienen la fiabilidad del diagnóstico.

### XIV. Validación de datos en todas las fronteras

Todo dato externo o no confiable DEBE validarse en tiempo de ejecución, en la frontera por la que entra y antes de usarse en cualquier lógica. Un tipo estático no es una validación.

- **Qué es una frontera:** cualquier punto por el que entra un dato que el código receptor no ha construido. Para ThrottleWatch, como mínimo:

  | Capa receptora | Fronteras |
  |---|---|
  | Rust | mensajes NDJSON del sidecar; parámetros de comandos Tauri (la WebView se trata como no confiable); ficheros CSV/JSON importados; filas de SQLite al migrar y preferencias leídas de la base; valores devueltos por Win32 (PDH, energía, idioma, monitores); variables de entorno |
  | TypeScript | respuestas y eventos del backend recibidos por el puente; `import.meta.env`; entrada del usuario en formularios antes de enviarla al backend |
  | .NET | comandos recibidos por `stdin`; valores y nombres de sensores devueltos por LibreHardwareMonitorLib; variables de entorno |

- **Herramienta por lenguaje:**
  - **TypeScript:** **Zod** es la biblioteca estándar y única de validación y parsing. Los tipos se infieren de los esquemas (`z.infer` / `z.output`); NO DEBE existir una interfaz escrita a mano que duplique la forma de un esquema.
  - **Rust:** deserialización tipada con `serde` (`deny_unknown_fields` en contratos), validación contra el JSON Schema canónico con `jsonschema` en las fronteras entre procesos y comprobaciones semánticas explícitas (rangos físicos, unidades, secuencia, tamaño).
  - **.NET:** `System.Text.Json` con tipos explícitos y comprobaciones semánticas; los comandos se aceptan solo si pertenecen a la lista permitida.
- **Sin aserciones de confianza:** NO DEBEN usarse `as T`, `as unknown as T`, el operador `!` ni tipos anotados sobre el resultado de `JSON.parse`, `invoke`, eventos, `import.meta.env` o cualquier otra frontera. ESLint DEBE aplicar `@typescript-eslint/consistent-type-assertions` con `assertionStyle: 'never'` y el conjunto `strict-type-checked` (reglas `no-unsafe-*`). En Rust están prohibidos `unwrap()` y `expect()` sobre datos de frontera.
- **Validación de sentido, no solo de forma:** un valor sintácticamente válido pero físicamente imposible (NaN, infinito, temperatura fuera de rango físico, marca temporal regresiva) se trata como dato ausente con su calidad, nunca como cero (principio X).
- **Doble validación:** lo que la interfaz valida para dar respuesta inmediata al usuario, el backend lo DEBE volver a validar; la validación de la interfaz nunca sustituye a la del backend.
- **Errores estructurados:** una validación fallida DEBE producir un error con código estable en inglés (`snake_case`), ruta del campo, motivo y, cuando proceda, la clave de catálogo del mensaje para el usuario. En TypeScript se usa `safeParse` y se devuelve un resultado discriminado (`{ ok: true, value } | { ok: false, error }`) en lugar de lanzar excepciones al cruzar el puente. La interfaz NO DEBE mostrar al usuario mensajes crudos de Zod, serde o .NET: los traduce desde el código de error. Los registros incluyen código y ruta, pero no el valor rechazado cuando pueda contener identificadores.
- **Cobertura de contratos:** toda frontera nueva DEBE llegar con su esquema y con pruebas que demuestren que acepta los casos válidos y rechaza, como mínimo, un campo ausente, un tipo incorrecto, un valor fuera de rango y un campo desconocido.

**Motivo:** el colector, los ficheros importados, la WebView y el sistema operativo pueden entregar datos corruptos, antiguos o manipulados; validar en la frontera impide que un dato erróneo se convierta en un diagnóstico erróneo.

### XV. Configuración y variables de entorno

ThrottleWatch es una aplicación de escritorio sin servidor: todo lo que se empaqueta es legible por quien lo instala. Por eso:

- **La aplicación instalada DEBE funcionar sin ninguna variable de entorno ni fichero `.env`.** Una versión de producción no lee `.env` ni permite que una variable de entorno cambie su comportamiento. La configuración del usuario vive en SQLite (principio X) y los parámetros del motor, en configuración versionada (principio VII).
- **Clases de variables permitidas:**

  | Clase | Prefijo o nombre | Dónde se leen | ¿Puede ser secreta? |
  |---|---|---|---|
  | Pública de compilación (interfaz) | `PUBLIC_*` (Vite con `envPrefix: 'PUBLIC_'`) | solo en el módulo `env.ts` de la interfaz; se incrustan en el paquete | **Nunca** |
  | Secreta de compilación y publicación | nombres que exige la herramienta, p. ej. `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` y las de Authenticode | solo en el almacén de secretos de CI o en el entorno del sistema de quien publica | Sí; NUNCA en el repositorio, en `.env` ni en código de la aplicación |
  | Solo desarrollo (Rust y .NET) | `TW_DEV_*` | solo en compilaciones de depuración (`cfg(debug_assertions)` o `#if DEBUG`) y en la compilación de pruebas `e2e` de CI (característica de Cargo `e2e` y configuración `E2E` de .NET), que nunca se distribuye; se eliminan de la versión de producción | No |

- **Acceso centralizado:** está prohibido `process.env` en todo el código de la aplicación. En la interfaz, `import.meta.env` solo PUEDE leerse en un único módulo de configuración que lo valida con Zod y exporta un objeto tipado e inmutable. En la configuración de compilación (`vite.config.ts` y scripts), las variables se leen con `loadEnv` de Vite y se validan con el mismo esquema. En Rust, `std::env::var` solo en el módulo de configuración; en .NET, `Environment.GetEnvironmentVariable` solo en su equivalente.
- **Fallo inmediato:** una variable obligatoria ausente o inválida DEBE detener la compilación, o el arranque de una versión de depuración, con un mensaje que indique el nombre, el motivo y la referencia a `.env.example`. Nunca se sustituye en silencio por un valor por defecto.
- **Nada de lo que protege al usuario es configurable por entorno:** endpoint del actualizador, activación de red o telemetría, límites de seguridad de la prueba guiada, parámetros del motor, rutas de datos, nivel de registro y la comprobación de firma del sidecar NO DEBEN poder cambiarse mediante variables de entorno ni argumentos de línea de órdenes en producción.
- **Ficheros:** `.env.example` se confirma en el repositorio y documenta cada variable (nombre, clase, capa que la lee, si es obligatoria y un valor ficticio). `.env` y `.env.*` están ignorados por Git. Añadir, renombrar o eliminar una variable exige actualizar en la misma PR `.env.example`, el esquema Zod o el módulo de configuración correspondiente y el plan afectado.

**Motivo:** en una aplicación de escritorio no existe un «lado privado» donde esconder secretos, y una variable de entorno capaz de desviar el actualizador o desactivar un límite de seguridad sería una puerta trasera.

### XVI. Verificación ejecutada y trazable

- **`pnpm check` obligatorio:** el espacio de trabajo DEBE proporcionar un comando reproducible `pnpm check` que ejecute `svelte-check --tsconfig <tsconfig real de la aplicación> --fail-on-warnings` y la comprobación de tipos de la configuración de compilación (`tsc --noEmit -p tsconfig.node.json`). Como el proyecto usa Svelte + Vite sin SvelteKit, no existe paso `svelte-kit sync`; si SvelteKit se adoptara por enmienda, ese paso DEBERÍA preceder a `svelte-check`.
- **Cuándo es obligatorio:** en todo cambio que afecte a componentes `.svelte`, ficheros `.ts` o `.svelte.ts`, propiedades o contratos de componentes, `tsconfig*.json`, `vite.config.ts`, `svelte.config.js`, la copia del sistema de diseño o los esquemas Zod. El harness de `design/` mantiene su propio `check`, que CI también ejecuta.
- **Orden del pipeline:** instalación con archivos de bloqueo → formato y lint → `pnpm check` → pruebas unitarias y de componentes → compilación → pruebas E2E. El mismo criterio se aplica en Rust (`cargo fmt --check` y `clippy -D warnings` antes de las pruebas) y en .NET (`dotnet format --verify-no-changes` y compilación sin advertencias antes de las pruebas). Un paso fallido detiene los siguientes.
- **Terminado significa cero:** una implementación no está terminada si `svelte-check`, `tsc`, ESLint, `clippy` o los analizadores de .NET informan de errores o advertencias.
- **Supresiones:** `@ts-ignore` y `@ts-nocheck` están prohibidos. `@ts-expect-error`, `<!-- svelte-ignore -->`, `eslint-disable`, `#[allow(...)]` y `#pragma warning disable` solo se admiten en línea, con justificación y referencia a una incidencia.
- **Honestidad de la verificación:** las personas y los agentes de IA DEBEN ejecutar realmente las comprobaciones antes de declararlas superadas y registrar el resultado (orden ejecutada, código de salida y número de errores y advertencias) en la PR o en el registro de la tarea. Si una comprobación no pudo ejecutarse, se DEBE decir explícitamente; NUNCA se afirma que una validación ha pasado sin haberla ejecutado.
- **Fallos preexistentes:** un fallo que ya existía antes del cambio DEBE documentarse explícitamente en la PR con su incidencia. No PUEDE ocultarse, suprimirse ni ignorarse, y tampoco corregirse fuera del alcance del cambio sin autorización de quien es responsable de la tarea.

**Motivo:** una comprobación que no se ejecuta o cuyo resultado no se registra no aporta ninguna garantía, y un «ya pasaba antes» sin documentar se convierte en deuda invisible.

### XVII. Registro (logging) y depuración

**Una única API de registro por capa.** El código de las funcionalidades NO DEBE depender directamente de ninguna biblioteca de registro; usa exclusivamente el envoltorio de su capa, que es el único lugar donde se configura la biblioteca:

| Capa | Biblioteca (oculta tras el envoltorio) | Envoltorio: única API permitida | Prohibido fuera del envoltorio |
|---|---|---|---|
| Interfaz (TypeScript) | `loglevel` | módulo `logging` del puente: `createLogger(scope)` devuelve `error`, `warn`, `info`, `debug` y `trace` | `console.*`, `debugger`, importar `loglevel` (ESLint `no-console` y `no-restricted-imports`) |
| Backend (Rust) | `tracing`, `tracing-subscriber`, `tracing-appender` | macros del módulo `logging` (`log_error!` … `log_trace!`), que exigen un código de evento | `println!`, `eprintln!`, `dbg!`, macros de `tracing` y tipos de `tracing_subscriber` (Clippy `print_stdout`, `print_stderr`, `dbg_macro` y `disallowed_macros`) |
| Colector (.NET) | `Microsoft.Extensions.Logging` | clase estática única `Log` con métodos generados por `[LoggerMessage]`, uno por evento | `Console.Write*`, `Debug.Write*`, `Trace.Write*` (`BannedApiAnalyzers`) |

El colector NO DEBE escribir ficheros: emite sus eventos como JSON por `stderr` y el backend los valida (principio XIV) y los escribe en el mismo fichero con `component: "agent"`. La interfaz reenvía sus eventos al backend mediante un único comando Tauri (`log_frontend`) validado y limitado a 60 eventos por minuto.

**Niveles.** Los cinco niveles tienen el mismo significado en todas las capas (`Critical` de .NET se registra como `error`):

| Nivel | Significado | Ejemplos |
|---|---|---|
| `error` | Una operación falló y afecta al usuario o puede perder datos | colector caído tras agotar reintentos, base de datos dañada, exportación o importación fallida, disco lleno |
| `warn` | Anomalía recuperada o funcionamiento degradado | mensaje del colector rechazado, reintento, sensor crítico perdido, parada de seguridad de la prueba guiada, geometría corregida |
| `info` | Hito del ciclo de vida, de bajo volumen | arranque y cierre con versiones, colector conectado y nivel de cobertura, inicio y fin de sesión, migración aplicada, comprobación de actualización y resultado, activación del registro detallado |
| `debug` | Detalle para diagnosticar un problema concreto | resumen de mensajes del protocolo, tiempos de escritura por lotes, reglas evaluadas por el motor en cada ventana |
| `trace` | Detalle por muestra o por mensaje | cada muestra y cada línea NDJSON; solo en desarrollo |

**Nivel efectivo.**

- **Producción, de fábrica:** `info` en el backend y el colector; la interfaz reenvía solo `warn` y `error`. El ruido en producción DEBE ser mínimo: ningún evento por muestra y, como objetivo, menos de 100 líneas por hora de monitorización sin incidencias.
- **Producción, «Registro detallado»:** interruptor en Ajustes › Acerca de y ayuda › Avanzado que sube todas las capas a `debug`. Se desactiva solo a las 24 horas o al reiniciar la aplicación (lo que ocurra antes), su estado es visible mientras dure y su activación y desactivación se registran en `info`. `trace` NUNCA está disponible en producción.
- **Desarrollo:** `debug` por defecto; `TW_DEV_LOG_LEVEL` (backend y colector) y `PUBLIC_LOG_LEVEL` (interfaz) admiten `error|warn|info|debug|trace`, se validan con el esquema de configuración (principio XV) y solo actúan en compilaciones de desarrollo. Una compilación de producción con `PUBLIC_LOG_LEVEL` definida falla. `RUST_LOG` no se lee nunca.
- **Pruebas:** silencio por defecto; las pruebas que verifican registros usan un sumidero en memoria del envoltorio.

**Formato.**

- **Fichero (orientado a máquina):** JSON por líneas en UTF-8, un evento por línea, validado por el esquema `log-event` de `packages/contracts/`. Campos: `ts` (UTC, ISO 8601 con milisegundos y `Z`), `level`, `component` (`ui`, `core` o `agent`), `target` (módulo), `code` (código de evento estable en inglés, `snake_case`), `msg` (en inglés), `session_id` y `protocol_version` cuando existan, `fields` (objeto) y `err` (cadena de causas) en los errores.
- **Salida para personas (consola de desarrollo y visor `pnpm logs:view` de los ficheros):** fecha y hora en formato español `dd/MM/yyyy HH:mm:ss,SSS` en la zona `Europe/Madrid`, con su abreviatura para evitar la ambigüedad del cambio de hora (`18/09/2026 14:03:07,512 CEST`), seguida de nivel, componente, `target`, código y mensaje. Nunca marcas Unix en bruto. El backend formatea también los eventos del colector, de modo que existe un único formateador por lenguaje (`jiff` con la base de zonas empaquetada en Rust e `Intl.DateTimeFormat('es-ES', { timeZone: 'Europe/Madrid' })` en TypeScript), probado en los dos cambios de hora anuales.
- El formato (`json` o `pretty`) y la zona horaria de la salida humana solo son configurables en desarrollo, y únicamente a través del envoltorio.
- Esta regla afecta solo a la salida para desarrolladores: la información técnica que ve el usuario sigue su idioma de interfaz y su zona horaria (HU-09).

**Contenido prohibido.** Ningún registro, en ningún nivel, PUEDE contener:

- secretos, claves, contraseñas, tokens, material de firma ni el nonce del canal con el colector (el `session_id` de monitorización, UUID local aleatorio, sí está permitido);
- nombre del equipo o del usuario, números de serie, direcciones MAC o IP, identificador del plan de energía, títulos de ventanas o nombres de procesos de otras aplicaciones, ni rutas absolutas con el perfil del usuario (se escriben como `%LOCALAPPDATA%\…` o `~\…`);
- contenido de ficheros importados, cargas completas de mensajes ni valores rechazados por una validación (se registran tamaño, código y ruta del campo).

El envoltorio de cada capa aplica una función de redacción basada en la misma lista permitida que la exportación anónima (HU-07), cubierta como módulo crítico (principio XIII).

**Volumen, rendimiento y ciclo de vida.**

- Escritura no bloqueante: registrar NUNCA bloquea la interfaz, el muestreo ni el motor. Si el búfer se llena, se descartan eventos y se registra el número descartado.
- Un mismo código repetido en menos de 60 segundos se agrupa en una sola línea con contador.
- Rotación de 5 ficheros de 5 MB. El espacio de los registros se muestra junto al de la base de datos. «Eliminar todos mis datos» y «Restablecer ThrottleWatch» borran los registros, y con la retención «solo esta sesión» se eliminan al salir.
- Los pánicos de Rust y las excepciones no controladas de .NET y de la interfaz se capturan y se registran en `error` con su código antes de terminar o recuperarse.

**Depuración.**

- Las DevTools de WebView2 y el puerto de depuración remota solo existen en compilaciones de desarrollo y en las compilaciones de prueba E2E de CI; nunca en la versión publicada.
- No se confirma código con `dbg!`, `debugger`, `console.*`, `Debug.WriteLine` ni registros temporales ajenos al envoltorio.
- El resumen técnico copiable de Ajustes › Acerca de NO DEBE incluir registros en bruto; el usuario decide si adjunta la carpeta de registros.

**Motivo:** los registros son la única ventana a lo que ocurre en el equipo de un usuario sin telemetría; deben ser útiles para diagnosticar, uniformes entre las tres capas y, a la vez, incapaces de revelar quién es el usuario o de degradar la medición.

## Orden de prioridad ante conflictos

Cuando dos objetivos choquen, prevalece el de número menor. La decisión y el objetivo sacrificado DEBEN documentarse en `plan.md` o en un ADR.

1. **Seguridad** del equipo y del usuario (principio VIII).
2. **Privacidad** y control de los datos (principios V y X).
3. **Veracidad del diagnóstico**: no afirmar más de lo que la evidencia permite (principios I y III).
4. **Accesibilidad** (principio XII).
5. **Resiliencia**: seguir siendo útil cuando algo falla (principio II).
6. **Consumo y rendimiento** dentro de los presupuestos de `spec.md`.
7. **Estética**: vidrio, animaciones y efectos visuales.
8. **Alcance y plazo**: se recorta funcionalidad antes que cualquiera de los puntos anteriores.

Ejemplos: si el efecto de vidrio compromete el contraste, se reduce el vidrio; si una fecha de entrega exige saltarse la puerta de accesibilidad, se retrasa la entrega o se recorta la funcionalidad.

## Plataforma y stack obligatorio

### Plataforma objetivo

- **Sistema operativo:** Windows 11 24H2 y 25H2 de 64 bits (plataforma principal) y Windows 10 22H2 de 64 bits mientras WebView2 y .NET 10 lo admitan oficialmente. Otras versiones no están soportadas.
- **Arquitectura:** solo x64 en el MVP. ARM64 requiere enmienda tras validar LibreHardwareMonitorLib y el acceso de bajo nivel en esa arquitectura.
- **Hardware:** una CPU x86-64 Intel o AMD por equipo, un usuario de Windows y temperaturas en grados Celsius.
- **Motor web:** WebView2 Runtime Evergreen del sistema. El instalador usa el modo `embedBootstrapper` de Tauri; es la única situación de red fuera del actualizador y solo ocurre si WebView2 falta en el equipo.

### Versiones fijadas

Versiones de partida verificadas el 2026-09-18. Los archivos de bloqueo son la fuente exacta vigente; la «Política de dependencias y versiones» regula cómo cambian.

**Toolchains**

| Herramienta | Versión | Dónde se fija |
|---|---|---|
| Rust (edición 2024, con `rustfmt` y `clippy`) | 1.98.1 | `rust-toolchain.toml` |
| .NET SDK / runtime (LTS) · C# | 10.0.401 / 10.0.12 · C# 14 | `global.json` (`rollForward: latestPatch`) |
| Node.js (LTS «Krypton») | 24.21.0 | `.nvmrc` y `engines` |
| pnpm | 12.4.2 | campo `packageManager` |
| SQLite (empaquetado por `libsqlite3-sys` 0.38.2) | 3.53.2 | característica `bundled` de `rusqlite` |
| Runner de CI | GitHub Actions `windows-2025` | flujos de `.github/workflows/` |

**Interfaz (TypeScript/Svelte)**

| Paquete | Versión |
|---|---|
| `svelte` | 5.57.0 |
| `typescript` | 6.0.3 |
| `vite` | 8.3.0 |
| `@sveltejs/vite-plugin-svelte` | 7.3.0 |
| `svelte-check` | 4.7.6 |
| `@tsconfig/svelte` | 5.0.8 |
| `zod` | 4.6.5 |
| `loglevel` | 1.9.2 |
| `@tauri-apps/api` | 2.11.1 |
| `@tauri-apps/cli` | 2.11.4 |
| `@tauri-apps/plugin-notification` | 2.4.0 |
| `@tauri-apps/plugin-autostart` | 2.5.1 |
| `@tauri-apps/plugin-updater` | 2.11.0 |
| `@tauri-apps/plugin-dialog` | 2.7.3 |
| `@tauri-apps/plugin-opener` | 2.5.5 |

TypeScript 7 no se adopta hasta que `svelte-check` y `typescript-eslint` lo admitan (hoy aceptan como máximo la 6.0).

**Backend (Rust)**

| Crate | Versión | Uso |
|---|---|---|
| `tauri` / `tauri-build` | 2.11.5 / 2.6.3 | shell de escritorio |
| `tauri-plugin-notification` | 2.4.0 | notificaciones de Windows |
| `tauri-plugin-autostart` | 2.5.1 | inicio con Windows |
| `tauri-plugin-updater` | 2.11.0 | actualizador; verifica firmas minisign (Ed25519) |
| `tauri-plugin-single-instance` | 2.4.4 | instancia única |
| `tauri-plugin-dialog` | 2.7.3 | diálogos de archivo nativos |
| `tauri-plugin-opener` | 2.5.5 | abrir la documentación en el navegador |
| `rusqlite` (`bundled`) | 0.40.2 | acceso a SQLite |
| `rusqlite_migration` | 2.6.0 | migraciones versionadas |
| `serde` / `serde_json` | 1.0.229 / 1.0.151 | serialización |
| `jsonschema` | 0.56.0 | validación de contratos en ejecución |
| `tokio` | 1.53.1 | ejecución asíncrona |
| `windows` | 0.62.2 | Win32: PDH, energía, idioma, monitores |
| `thiserror` | 2.0.20 | tipos de error |
| `tracing` / `tracing-subscriber` / `tracing-appender` | 0.1.44 / 0.3.23 / 0.2.5 | registros estructurados y rotados |
| `time` | 0.3.55 | fechas UTC |
| `jiff` | 0.2.37 | formato humano de registros en `Europe/Madrid` (base de zonas empaquetada) |
| `uuid` | 1.26.1 | identificadores de sesión |
| `csv` | 1.4.0 | exportación CSV |
| `minisign-verify` | 0.2.5 | verificación de la firma minisign (Ed25519) del manifiesto de release en el lanzador elevado (ADR-0004 R2/C3); es el crate que `tauri-plugin-updater` usa internamente |

**Sidecar (.NET)**

| Paquete | Versión | Uso |
|---|---|---|
| `LibreHardwareMonitorLib` (MPL-2.0) | 0.9.6 | sensores Intel/AMD |
| `System.Text.Json` | incluido en .NET 10.0.12 | protocolo NDJSON |
| `Microsoft.Extensions.Logging` / `Microsoft.Extensions.Logging.Console` | 10.0.12 / 10.0.12 | registro JSON por `stderr` |
| `Microsoft.CodeAnalysis.BannedApiAnalyzers` | 5.6.0 | prohibición de `Console.Write*` y similares |

Destino `net10.0-windows`, publicación autocontenida `win-x64`, `Nullable` activado y advertencias tratadas como errores. NativeAOT solo se adopta si un spike demuestra compatibilidad con LibreHardwareMonitorLib.

**Pruebas, calidad y licencias**

| Herramienta | Versión | Componente |
|---|---|---|
| `cargo-nextest` | 0.9.145 | ejecución de pruebas Rust |
| `cargo-llvm-cov` | 0.9.1 | cobertura Rust |
| `proptest` | 1.11.0 | pruebas de propiedades del motor |
| `insta` | 1.48.0 | instantáneas de informes |
| `cargo-deny` | 0.20.2 | licencias, avisos de seguridad y duplicados |
| `cargo-about` | 0.9.2 | avisos de terceros Rust |
| `vitest` / `@vitest/coverage-v8` | 5.0.1 / 5.0.1 | pruebas y cobertura TS |
| `@testing-library/svelte` | 5.4.2 | pruebas de componentes |
| `@testing-library/jest-dom` | 7.0.1 | aserciones DOM |
| `@testing-library/user-event` | 14.6.7 | interacción simulada |
| `jsdom` | 30.1.0 | entorno DOM de Vitest |
| `@playwright/test` | 1.63.0 | pruebas de UI y E2E (WebView2 vía CDP) |
| `@axe-core/playwright` / `axe-core` | 4.13.0 / 4.13.0 | accesibilidad automatizada |
| `ajv` | 8.20.0 | validación del JSON Schema canónico en pruebas de contrato TS (no se usa en la aplicación) |
| `eslint` | 10.10.0 | análisis estático TS/Svelte |
| `typescript-eslint` | 8.70.0 | reglas TypeScript |
| `eslint-plugin-svelte` | 3.23.0 | reglas Svelte |
| `prettier` / `prettier-plugin-svelte` | 3.9.8 / 4.1.1 | formato TS/Svelte |
| `xunit.v3` / `xunit.runner.visualstudio` | 4.0.1 / 4.0.0 | pruebas del sidecar |
| `Microsoft.NET.Test.Sdk` | 18.10.1 | ejecución de pruebas .NET |
| `coverlet.collector` | 10.0.1 | cobertura C# |
| `Shouldly` | 4.3.0 | aserciones C# |
| `JsonSchema.Net` | 9.4.0 | validación de contratos en C# |

Verificación manual de accesibilidad con Narrador de la versión de Windows probada y con la última versión estable de NVDA; la versión usada se registra en el informe de verificación.

**Empaquetado y distribución**

- Instalador NSIS generado por el bundler de Tauri 2.11, instalación por usuario y sin elevación.
- El acceso de bajo nivel (proveedor usado por LibreHardwareMonitorLib 0.9.6), si se aprueba, se instala en un paso separado por máquina con UAC explícito.
- Artefactos de actualización firmados con minisign (Ed25519) mediante `tauri-plugin-updater`; binarios e instalador firmados con Authenticode antes de cualquier distribución pública.

### Tecnologías prohibidas

Sin enmienda previa NO DEBEN incorporarse:

- Electron, SvelteKit u otro framework de aplicación sobre Svelte + Vite.
- Kits de componentes o de CSS (Tailwind, Bootstrap, Material, etc.), bibliotecas de gráficos (ECharts, Chart.js, D3 completo) o de iconos: la interfaz usa el sistema de diseño propio y `AnalysisChart`.
- Bibliotecas externas de internacionalización: se usan catálogos propios `es`/`en` y la API `Intl`.
- Bibliotecas de validación distintas de Zod en TypeScript (Yup, Valibot, io-ts, ArkType, class-validator, etc.).
- Lectura de ficheros `.env` en tiempo de ejecución en versiones de producción (`dotenv`, `dotenvy` u otras) y uso de `process.env` en el código de la aplicación.
- Otras bibliotecas de registro (Pino, winston, bunyan, `log`/`env_logger`, Serilog, NLog, log4net) y cualquier destino de registros fuera del equipo.
- ORM en Rust (Diesel, SeaORM) o bases de datos distintas de SQLite.
- Cualquier SDK de telemetría, analítica o informes de fallo (Sentry, Application Insights, etc.).
- Clientes HTTP en la interfaz, fuentes o recursos remotos (CDN) y contenido web remoto en la WebView.
- WinRing0 o cualquier controlador sin firma válida, HWiNFO u otras herramientas externas de sensores.
- `Newtonsoft.Json` y `FluentAssertions` 8 o posterior (licencia comercial).
- Criptografía propia: la verificación de firmas la hace `tauri-plugin-updater`.
- Dependencias con licencias incompatibles con GPL-3.0.

### Política de dependencias y versiones

- Se confirman en el repositorio `Cargo.lock`, `pnpm-lock.yaml` y `packages.lock.json`; CI instala con `--locked`, `--frozen-lockfile` y `--locked-mode`. Los manifiestos npm usan versiones exactas (sin `^` ni `~`).
- **Parche** (x.y.**z**): se actualiza mediante PR con CI verde, sin enmienda.
- **Menor** (x.**y**.z): PR que actualiza la tabla de esta constitución (enmienda PATCH) y comprueba notas de versión.
- **Mayor** (**x**.y.z) o sustitución de una tecnología: ADR en `docs/adr/` y enmienda MINOR.
- Vulnerabilidades: las críticas y altas se corrigen en 7 días naturales; las medias, en 30.
- Tauri 3 y .NET 11 no se adoptan hasta publicar versión estable y superar una evaluación registrada en ADR.
- `design/harness/` y `design/mockup/` son proyectos independientes con su propio archivo de bloqueo npm, pero DEBEN usar las mismas versiones de Svelte, Vite y TypeScript que la aplicación.

## Estándares de ingeniería

- Código, contratos, eventos de diagnóstico y migraciones de datos con versiones explícitas.
- Formato y análisis estático obligatorios y sin advertencias: `rustfmt` y `clippy -D warnings` en Rust; ESLint, Prettier y `svelte-check` en TypeScript/Svelte; `dotnet format` y analizadores con `AnalysisLevel` `latest-recommended` en .NET.
- TypeScript en modo `strict` sin `any` implícito; `unsafe` en Rust solo en el módulo de interoperabilidad Win32, con comentario `SAFETY:` en cada bloque.
- Dependencias fijadas mediante archivos de bloqueo y revisión de licencias con `cargo-deny`, `cargo-about` y el informe equivalente de npm y NuGet.
- Binarios y actualizaciones firmados antes de distribución pública.
- Registros estructurados, rotados y sin datos sensibles, emitidos solo a través del envoltorio de cada capa (principio XVII).
- Ninguna cadena de sensor del proveedor puede convertirse directamente en lógica de negocio; debe pasar por la capa de normalización.
- Ningún valor ausente se representará como cero.
- Ningún texto visible se escribe literalmente en un componente: sale de los catálogos `es` y `en`, cuya igualdad de claves se comprueba en CI.
- Capacidades de Tauri (`capabilities/`) con lista permitida mínima y CSP restrictiva sin orígenes remotos; añadir un permiso exige revisión explícita en la PR.
- El sistema de diseño aprobado para el producto DEBE ser la fuente canónica de tokens, componentes y patrones de interacción. Una excepción o componente nuevo DEBE documentar su necesidad, reutilizar los tokens existentes y satisfacer las mismas exigencias de accesibilidad, localización, temas y adaptación de tamaño; no se permiten duplicados locales casi equivalentes.

### Idioma del proyecto

- Identificadores, comentarios de código, mensajes de registro, claves de contrato y claves de CSV/JSON: en inglés.
- Especificaciones, planes, ADR, mensajes de commit y descripciones de PR: en español.
- Interfaz: catálogos completos en español e inglés.

## Puertas de calidad

Una característica no se considera terminada si incumple cualquiera de estas puertas:

1. Requisitos y escenarios de aceptación enlazados a pruebas.
2. Contratos compatibles o migración documentada.
3. Pruebas unitarias y de integración relevantes superadas.
4. Verificación con al menos una traza Intel, una AMD y una degradada/sin sensores; para el motor de diagnóstico, además, el corpus etiquetado con razones directas y sus copias degradadas.
5. Accesibilidad básica: teclado, foco visible, contraste y modo de movimiento reducido. Además, `axe-core` sin infracciones de gravedad `serious` o `critical` en cada pantalla y estado afectado, y una revisión manual con teclado y Narrador antes de cada versión publicada.
6. Consumo en reposo y crecimiento de almacenamiento dentro de los presupuestos definidos.
7. Mensajes de diagnóstico revisados para no presentar inferencias como hechos.
8. Avisos y obligaciones de licencias de terceros incorporados al paquete.
9. Las pantallas afectadas se han verificado en ambos temas, ambos idiomas y los tamaños compacto, medio y expandido definidos por el producto, incluidos sus estados de carga, vacío, degradado y error aplicables.
10. Umbrales de cobertura del principio XIII cumplidos en los tres componentes.
11. Formato y análisis estático sin advertencias; instalación con archivos de bloqueo; versiones coincidentes con la tabla de esta constitución o con una actualización de parche permitida.
12. Ninguna petición de red observada con el actualizador apagado (prueba automatizada de tráfico cero).
13. `pnpm check` y las comprobaciones equivalentes de Rust y .NET ejecutados en el orden del principio XVI, sin errores ni advertencias, con el resultado registrado en la PR.
14. Cada frontera de datos nueva o modificada tiene su esquema y pruebas de aceptación y rechazo (principio XIV); cada variable de entorno nueva figura en `.env.example` y en el esquema de configuración (principio XV).
15. Ningún uso de registro fuera de los envoltorios (lints de la puerta 11); los eventos nuevos tienen código estable y pasan las pruebas de redacción y del esquema `log-event` (principio XVII).

## Gobernanza

Esta constitución prevalece sobre decisiones locales de implementación. Toda excepción DEBE documentarse en `plan.md` con motivo, alternativa descartada, riesgo y fecha de retirada. Las enmiendas requieren actualizar la versión, registrar el cambio y comprobar los documentos de especificación afectados.

- **Cumplimiento:** cada PR incluye una lista de comprobación de las puertas de calidad aplicables y quien revisa DEBE verificarla. La sección «Comprobación de la constitución» de cada `plan.md` DEBE cubrir todos los principios.
- **Conflictos:** se resuelven con el «Orden de prioridad ante conflictos»; si el conflicto es con la propia constitución, se enmienda antes de implementar.
- **Versionado de la constitución:** MAYOR cuando se elimina o redefine un principio de forma incompatible; MENOR cuando se añade un principio o sección, o se amplía materialmente una obligación; PARCHE para aclaraciones, erratas y actualizaciones menores de la tabla de versiones.

### Historial de enmiendas

- **1.1.0 (2026-09-17):** se incorpora el sistema de diseño aprobado como fuente canónica y se añade la matriz obligatoria de verificación visual y funcional.
- **1.2.0 (2026-09-18):** revisión del motor de diagnóstico: el principio III exige que las cifras de rendimiento procedan de una medición comparable o de un modelo físico explícito y considera la ventana de turbo y la gestión térmica del fabricante; el principio VIII acota la única escritura permitida en registros (limpieza de bits de estado); la puerta 4 incluye el corpus etiquetado. Documentos afectados: `spec.md`, `plan.md`, `research.md`, `data-model.md`, contratos, `ux-visual-spec.md`, `tasks.md`.
- **1.3.0 (2026-09-18):** se añaden los principios IX (separación de capas), X (almacenamiento), XI (usabilidad), XII (accesibilidad WCAG 2.2 AA) y XIII (testabilidad y cobertura 80 %/95 %); el orden de prioridad ante conflictos; la plataforma soportada; el stack obligatorio con versiones fijadas; las tecnologías prohibidas; la política de dependencias y el idioma del proyecto. Se amplían los principios V y VII, los estándares de ingeniería y las puertas de calidad 5, 10, 11 y 12. Documentos afectados: `plan.md`, `tasks.md`, `spec.md` (NFR-005).
- **1.4.0 (2026-09-18):** se añaden los principios XIV (validación de datos en todas las fronteras, con Zod como estándar en TypeScript), XV (configuración y variables de entorno) y XVI (verificación ejecutada y trazable, con `pnpm check` obligatorio); `zod` 4.6.5 entra en el stack; se amplían los principios IX y XIII, las tecnologías prohibidas y las puertas de calidad 13 y 14. Documentos afectados: `plan.md`, `tasks.md`, contratos.
- **1.5.0 (2026-09-18):** se añade el principio XVII (registro y depuración): un envoltorio por capa sobre `tracing`, `Microsoft.Extensions.Logging` y `loglevel`; cinco niveles comunes; `info` en producción con «Registro detallado» temporal desde Ajustes; variables de nivel solo en desarrollo; JSON UTC en fichero y formato humano español en `Europe/Madrid`; contenido prohibido y redacción; borrado de registros con los datos. Entran en el stack `loglevel`, `jiff`, `Microsoft.Extensions.Logging(.Console)` y `BannedApiAnalyzers`; se amplían los principios X y XIII, las tecnologías prohibidas, los estándares de ingeniería y la puerta de calidad 15. Documentos afectados: `spec.md`, `plan.md`, `tasks.md`, contratos.
- **1.5.1 (2026-09-18):** aclaración del principio XII: con escala 200 % en monitores cuya altura útil no alcanza 600 px lógicos, la ventana mínima pasa a 480×500. Documentos afectados: `spec.md` (escenario 8 de la historia 9, NFR-011, parámetro «Ventana»), `plan.md`, `research.md`, `data-model.md`, `ux-visual-spec.md`, `tasks.md`.
- **1.5.2 (2026-09-18):** aclaraciones derivadas de `/speckit-analyze`: el principio XV admite `TW_DEV_*` en la compilación de pruebas `e2e` de CI (nunca distribuida), en coherencia con el principio XVII; el principio XII fija el comportamiento con altura útil inferior a 500 px lógicos (ventana maximizada con desplazamiento). Documentos afectados: `spec.md`, `plan.md`, `tasks.md`, `ux-visual-spec.md`, `historias.md`.
- **1.5.3 (2026-09-19):** enmienda PARCHE autorizada por la persona propietaria: entra en la tabla de crates `minisign-verify` 0.2.5 (MIT, sin dependencias) para verificar la firma minisign del manifiesto de release en el lanzador elevado exigida por ADR-0004 (R2, C3); no cambia ningún principio ni puerta de calidad. Documentos afectados: `AGENTS.md`, `.agents/meta/detected-stack.yaml`, `apps/desktop/src-tauri/Cargo.toml`.
- **1.5.4 (2026-09-22):** enmienda PARCHE autorizada por la persona propietaria (T175): se retiran de la tabla de pila fijada `@tauri-apps/plugin-process` y `tauri-plugin-process` 2.3.1 («reinicio tras instalar»). El motivo original —que el backend pidiera un reinicio tras instalar una actualización— no aplica a la implementación real: el instalador NSIS relanza la aplicación él mismo (`docs/adr/0005-actualizador-firmado.md`), así que Rust nunca necesita ese plugin; mantenerlo habría documentado una dependencia que no se iba a añadir nunca. No cambia ningún principio ni puerta de calidad. Documentos afectados: `specs/001-cpu-thermal-diagnostics/tasks.md` (T175).

**Versión**: 1.5.4 | **Ratificada**: 2026-09-17 | **Última modificación**: 2026-09-22
