# Investigación y decisiones técnicas

## 1. Fuente de sensores

### Decisión

Integrar `LibreHardwareMonitorLib` en un sidecar .NET propio. No ejecutar la aplicación GUI de LibreHardwareMonitor ni depender de HWiNFO.

### Motivos

- API diseñada para recorrer hardware y sensores.
- Cobertura Intel y AMD sin implementar directamente registros de muchas generaciones.
- Temperatura, carga, reloj y potencia pueden coexistir en una fuente.
- Licencia MPL 2.0 compatible con una aplicación mayor de otra licencia, siempre que se cumplan sus obligaciones sobre los archivos cubiertos y las modificaciones.

### Riesgos

- Ningún proyecto garantiza todos los sensores en todo hardware.
- **Hipótesis de trabajo (a confirmar en T019):** `LibreHardwareMonitorLib` no expone reloj efectivo (solo reloj por núcleo multiplicador × bus), ni banderas PROCHOT/PL1/PL2/EDP como sensores, ni la clase de núcleo P/E/LP. Decisiones derivadas: frecuencia activa y base calculadas con contadores PDH en Rust; razones de limitación, límites de potencia y TCC offset leídos del MSR por el sidecar cuando el acceso de bajo nivel lo permite (nivel A); clasificación de núcleos por CPUID/`GetLogicalProcessorInformationEx` en el sidecar. Si el spike contradice la hipótesis, se simplifica; si la confirma y el MSR no es accesible sin UAC recurrente, se aplica la puerta de viabilidad de `plan.md` (fase 0).
- El acceso de bajo nivel, firma y redistribución del controlador debe validarse jurídicamente y en el instalador.
- Los nombres y sensores disponibles cambian por CPU, placa, permisos y versión.

### Alternativas consideradas

- **Intel PCM**: excelente complemento para Intel y métricas de rendimiento, pero no cubre AMD ni ofrece una abstracción única para el producto generalista.
- **WMI/contadores Windows**: útiles para carga y frecuencia/performance de SO; insuficientes para temperatura y señales profundas en muchos equipos.
- **HWiNFO Shared Memory**: gran cobertura, pero añade dependencia externa y condiciones de licencia/distribución menos adecuadas al objetivo.
- **Acceso MSR/SMU propio en Rust**: máximo control, coste y riesgo de mantenimiento demasiado altos para el MVP.

**Fuente primaria:** [Repositorio de LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor)

## 2. Frontera Tauri ↔ .NET

### Decisión

Empaquetar un binario .NET publicado de forma autocontenida como sidecar Tauri. Comunicación por `stdin/stdout` con NDJSON versionado y `stderr` reservado para logs.

### Motivos

- El proceso padre controla el ciclo de vida.
- No se abre un puerto local.
- Facilita pruebas con un sidecar falso y captura de trazas.
- El bloqueo o fuga del colector queda aislado de la WebView.

### Descartado

- **HTTP localhost**: más superficie, puertos y autenticación.
- **Named pipes desde el inicio**: válidas, pero más complejas; reconsiderar solo si stdio limita rendimiento o recuperación.
- **AOT e interoperabilidad dentro del proceso Rust**: complejidad elevada y mayor acoplamiento.

**Fuente primaria:** [Tauri v2 — Embedding External Binaries](https://v2.tauri.app/develop/sidecar/)

## 3. Frontend y gráficos

### Decisión

Svelte 5 + TypeScript + Vite con el sistema de diseño entregado en `design/`. La vista temporal utiliza `AnalysisChart`, un SVG propio que representa cuatro pistas sincronizadas, cursor común, huecos, calidad reducida, bandas de eventos y selección de rango. No se incorpora ECharts ni otra biblioteca de gráficos. CSS propio mediante tokens; no adoptar un kit visual que imponga apariencia genérica.

### Motivos

- Svelte reduce trabajo de sincronización en una UI de estado continuo.
- El componente entregado tiene API presentacional estable, navegación por teclado, alternativa textual/tabular y cero dependencias de ejecución adicionales.
- El benchmark del harness mantiene el primer render por debajo de 300 ms incluso muy por encima del presupuesto recomendado; la guía operativa fija 2.000–3.000 puntos por pista y 10.000–12.000 totales por vista.
- El mapa térmico de CPU es una cuadrícula Svelte separada; no necesita un motor de gráficos.

### Riesgo y mitigación

Un SVG puede consumir CPU y memoria si recibe sesiones crudas ilimitadas. Rust entrega ventanas ya agregadas a la resolución solicitada; conserva extremos, huecos, peor calidad material y límites de eventos. El detalle crudo se consulta bajo demanda y el zoom solicita una ventana más precisa. Se repetirá el benchmark en hardware objetivo con 4 pistas y 3.000 puntos por pista.

**Fuente de decisión:** `design/AGENTS.md`, sección `AnalysisChart: SVG vs. ECharts`, y harness reproducible de `design/harness/`.

## 4. Persistencia

### Decisión

SQLite gestionado por Rust, con WAL, migraciones y escritura por lotes.

### Motivos

- Local, transaccional, portable y consultable.
- Adecuado para 1 Hz y sesiones de una sola máquina.
- Facilita retención, resúmenes y exportación consistente.

### Descartado

- JSON por sesión como almacenamiento primario: sencillo al principio, malo para retención, búsquedas y actualizaciones atómicas.
- Base de datos embebida en el sidecar: mezcla adquisición con persistencia y dificulta reinicio/reproducción.

## 5. Qué significa “rendimiento perdido”

*Revisado el 2026-09-18.*

### Decisión

La aplicación no calcula «rendimiento perdido» comparando relojes con un baseline. Ofrece dos cosas distintas y rotuladas por separado:

1. **Potencial con mejor refrigeración** (monitorización pasiva, solo nivel A): cuánto subiría la frecuencia si el calor dejara de ser el límite. Se calcula con el **techo de potencia**: si la temperatura está en el límite y la potencia P queda por debajo del límite de potencia efectivo PL1, el hueco de potencia marca cuánto podría subir la frecuencia. Con P ∝ f·V² y V aproximadamente proporcional a f, P ∝ f³, así que `g = (PL1 / P)^(1/3) − 1`. Se acota por la frecuencia observada en la ventana de turbo con los mismos núcleos activos (el procesador no pasará de ahí) y se expresa como `[0,5·g, 1,0·g]`.
2. **Rendimiento medido** (solo diagnóstico guiado): el generador cuenta trabajo completado; se compara el sostenido con el inicial y se desglosa la diferencia por causa.

### Razón

La versión anterior (mediana de reloj actual / mediana de reloj del baseline) tenía cuatro fallos: comparaba contra baselines medidos a menudo durante el turbo PL2; los baselines aprendidos se seleccionaban en momentos de poca exigencia; la «carga %» no distingue tipos de trabajo (AVX, memoria), así que el baseline rara vez era comparable; y el rango, calculado con la dispersión estadística, ignoraba el error sistemático, que domina.

### Limitaciones conocidas

- La relación cúbica es de primer orden; cerca del máximo la curva tensión-frecuencia es más empinada (el potencial real es menor), y las cargas limitadas por memoria escalan menos que la frecuencia. El extremo inferior `0,5·g` cubre ambos efectos; la medición guiada permite contrastarlo.
- A igual potencia, un chip más caliente fuga más corriente y rinde algo menos. Por eso mejorar la refrigeración ayuda un poco (del orden del 1–3 %) incluso en limitación de potencia pura; la aplicación lo menciona sin cuantificarlo.
- Sin PL1 conocido no hay cifra.

## 6. Detección térmica

*Revisado el 2026-09-18.*

### Decisión

Motor basado en reglas explicables y deterministas que clasifica por **niveles de cobertura**:

- **Nivel A**: razones de limitación directas. Intel expone en `MSR_CORE_PERF_LIMIT_REASONS` bits separados para `THERMAL`, `PROCHOT`, `PL1`, `PL2`, `EDP`, `VR_TDC` y otros, con bits de registro que se limpian tras leer. Es la misma fuente que usan ThrottleStop y HWiNFO.
- **Niveles B/C**: inferencia por **mesetas**: la magnitud que limita se queda clavada en su tope. Temperatura plana en el límite → límite térmico; potencia plana con margen térmico → límite de potencia.

### Por qué mesetas y no «caída de reloj tras calor»

La regla inicial («margen ≤ 8 °C seguido de caída de reloj ≥ 8 %») reproduce exactamente la firma del fin del turbo en Intel: durante Tau (28–56 s) la CPU consume PL2, se calienta, y después baja a PL1. Sin la razón `PL1` a la vista, esa transición se habría clasificado como térmica en casi cualquier portátil. Además, detectar caídas no detecta estados: un equipo que ya empieza caliente o que se degrada lentamente nunca muestra la caída.

### Casos que obligaron a ampliar las clases

- **Gestión térmica del fabricante (Intel DTT/DPTF, AMD STAPM)**: el firmware baja PL1 según la temperatura del chasis. La CPU queda a 70–80 °C, la potencia clavada y la frecuencia baja. Por las razones parece «potencia», pero la causa es térmica del equipo y **mejorar la ventilación sí ayuda**. Se detecta porque el límite de potencia efectivo (o el nivel de la meseta de potencia) baja durante la sesión sin cambio de plan ni de alimentación. Clase `platform_limited · chassis_thermal`.
- **PROCHOT externo**: el controlador del portátil puede activar PROCHOT# por batería, cargador o VRM, sin relación con la temperatura de la CPU. Intel lo separa del bit `THERMAL`. Clase `platform_limited · external_prochot`; la recomendación es revisar la alimentación.
- **Boost que opera en el límite térmico por diseño**: Zen 3/4/5 de sobremesa y los Intel de sobremesa con límites abiertos suben hasta el límite térmico y convierten el margen en frecuencia. Estar «limitado térmicamente» ahí es normal. La **frecuencia base** (la garantizada) separa lo informativo (`boost`) de lo problemático (`below_base`).
- **TCC offset**: muchos portátiles adelantan la activación térmica (TjMax 100 °C, límite real 90 °C). Sin leer el offset, el margen es falso.

### Regla de comunicación

- Razón `THERMAL` → «confirmada»; gravedad según la frecuencia base.
- Meseta térmica sin razones → «compatible con limitación térmica».
- Solo calor, o calor dentro de la ventana de turbo → «temperatura alta; pérdida no demostrada».
- Meseta o razón de potencia con margen → «limitada por potencia».
- Límite de potencia descendente o PROCHOT sin THERMAL → «limitada por el equipo».
- Caída sin meseta ni razón → «indeterminada», citando como alternativas la gestión de energía de Windows (EcoQoS, EPP, modo eficiencia) y el plan energético.

## 7. Referencias y comparaciones

*Revisado el 2026-09-18: se eliminan los baselines aprendidos.*

### Decisión

La única referencia es el **resultado medido de un diagnóstico guiado**, marcado por el usuario. Dos resultados son comparables si coinciden CPU, perfil de duración, contexto energético y versión del generador. La comparación «antes/después» usa el rendimiento medido sostenido, no relojes.

### Motivo

Un baseline aprendido de la monitorización pasiva exige que la carga actual y la de referencia sean del mismo tipo, y la aplicación no puede saberlo: dos cargas con el mismo porcentaje de uso pueden tener frecuencias muy distintas (AVX, memoria, pocos hilos). Con el generador propio, la carga es idéntica por construcción.

## 8. Prueba de carga

### Decisión pendiente de spike

Preferencia inicial: carga integrada, progresiva y cancelable, únicamente si puede aislarse y detenerse con garantías. Alternativa: modo guiado que observa una carga externa del usuario; en ese caso no hay rendimiento medido y el informe solo usa el potencial por techo de potencia.

### Condiciones para aprobar carga integrada

- Watchdog fuera de los workers.
- Cancelación en menos de 500 ms.
- Parada ante pérdida del proceso padre, sensor crítico o las condiciones de FR-085. **No** se para al alcanzar el límite térmico: es el fenómeno que se mide y el procesador se protege solo.
- Bucle de trabajo fijo y versionado que cuenta operaciones por hilo; sin AVX-512; AVX2 solo en un perfil documentado.
- Duración de la carga sostenida suficiente para que los últimos 120 s queden fuera de Tau.
- Mensaje explícito de que no es un benchmark homologado: mide este equipo frente a sí mismo.

## 9. Privilegios y controlador

### Decisión

Instalar acceso de bajo nivel únicamente cuando sea necesario, mediante acción separada. Ejecutar la aplicación y sidecar sin elevar siempre que el controlador permita la lectura requerida. Cobertura reducida antes que elevación silenciosa.

### Preguntas del spike

1. ¿Qué sensores se pierden sin controlador y sin administrador en la matriz objetivo?
2. ¿Es redistribuible el instalador/controlador en cada arquitectura?
3. ¿Puede una instalación por máquina permitir lectura posterior a usuario estándar?
4. ¿Qué mecanismos de desinstalación y actualización exige?

No se cerrará el diseño del instalador hasta responderlas.

## 10. Compatibilidad y soporte

### Decisión

Publicar “cobertura observada” y no “compatibilidad total”. La aplicación genera una ficha local:

- identificación correcta;
- temperatura disponible;
- límite/margen disponible;
- frecuencia activa y frecuencia base;
- límite térmico efectivo (TjMax y TCC offset);
- límites de potencia efectivos y razones de limitación;
- nivel de cobertura (A/B/C);
- potencia;
- indicadores térmicos/eléctricos;
- confianza máxima alcanzable.

Esto permite que una CPU antigua siga obteniendo valor aunque no admita una cuantificación completa.

## 11. Localización y selección de idioma

### Decisión

Catálogos completos `es` y `en`, con claves semánticas compartidas y sin literales visibles en componentes. `system` resuelve por la etiqueta BCP 47 de Windows: prefijos `es`, `ca`, `gl`, `eu`, `ast` y `an` usan español —incluido `ca-AD`— y cualquier otro prefijo, incluido `pt`, usa inglés. La selección manual prevalece.

El idioma del sistema se toma del **primer** idioma de la lista de idiomas de visualización de Windows (`GetUserPreferredUILanguages`), no de la región. Números y fechas se formatean con `Intl` según el idioma efectivo; solo °C.

### Motivos

- Cumple la política de producto para lenguas de regiones ibéricas sin mantener traducciones parciales.
- Un fallback único a inglés evita mezclar idiomas.
- Claves técnicas inglesas en CSV/JSON mantienen contratos estables.

## 12. Tema, barra propia y estado de ventana

### Decisión

La ventana Tauri se publica sin decoraciones y monta un componente propio inspirado en el patrón validado de SmartDisk: presentación desacoplada de la API de ventana, región de arrastre, doble clic y tres controles. El tema usa tokens claros/oscuros y `system` escucha el modo de aplicación de Windows (`AppsUseLightTheme`). La geometría se persiste en coordenadas lógicas (mínimo 480×600, inicial 1100×760) y se valida contra monitores activos antes de mostrar la ventana.

### Salvaguardas

No se persiste minimizado; se conserva el último rectángulo restaurado al cerrar maximizado; una geometría fuera de pantalla se recentra; controles, doble clic y drag region tienen pruebas de teclado y puntero.

## 13. Primer inicio y configuración

### Decisión

Recorrido versionado de cinco diapositivas con progreso persistente, omisión y repetición manual. La detección pasiva comienza en la quinta. Las preferencias tipadas centralizan idioma, tema, movimiento, muestreo, retención, bandeja, inicio con Windows, privacidad y actualizaciones.

### Motivos

- Separa educación de consentimiento: leer una diapositiva nunca solicita elevación.
- Versionar recorrido y novedades evita repetir contenido innecesario.
- Perfiles comprensibles evitan exponer milisegundos a usuarios corrientes.

## 14. Actualizaciones

### Decisión

Actualizador opcional mediante el plugin oficial de Tauri y un endpoint fijo de GitHub Releases, sin canales. Está apagado de fábrica; al activarlo, comprueba automáticamente como máximo cada 24 horas (al arrancar si han pasado y con temporizador en ejecución). La comprobación manual puede omitir esa espera. Comprobar, descargar e instalar son gestos independientes y el artefacto se verifica con Ed25519 antes de declararlo instalable. Instalador NSIS por usuario sin elevación; el acceso de bajo nivel, si se aprueba, se instala en un paso separado por máquina.

### Restricciones

La publicación produce manifiesto y artefactos firmados; instalar se bloquea durante diagnóstico o exportación; no se envían identificadores; desactivar purga cualquier estado o artefacto pendiente del actualizador.

## 15. Validación del motor con corpus etiquetado

*Añadido el 2026-09-18.*

### Decisión

Los criterios SC-003 a SC-005 y SC-016 a SC-018 se miden en CI con un corpus de trazas cuya **verdad de referencia** son los bits de razón de Intel, no una etiqueta manual.

### Protocolo

1. Grabar trazas de nivel A (acceso de bajo nivel instalado) en la matriz de hardware: Intel híbrido, Intel anterior, portátil con DTT/DPTF, sobremesa con límites abiertos, y AMD Zen 4 con tabla PM permitida. Cargas: render multihilo, compilación, juego limitado por CPU (pocos núcleos), carga AVX2 y reposo con calor residual.
2. Etiquetar cada ventana estable con la clase que dictan los bits (`THERMAL`, `PROCHOT`, potencia, corriente), la tendencia de PL1 y la frecuencia base (gravedad). Las trazas sintéticas se marcan como tales.
3. Generar copias **degradadas**: B (se eliminan razones, límites y TCC offset) y C (además, potencia). Así se mide cuánto acierta la inferencia sin driver frente a la verdad de nivel A del mismo equipo y la misma carga.
4. Casos obligatorios: fin de turbo sin calor, equipo que empieza caliente, degradación lenta, DPTF, PROCHOT externo, Zen 4 en su límite por diseño, juego con pocos núcleos, EcoQoS.
5. Calibrar los pesos de la confianza con este corpus y versionar el resultado con el ruleset.

### Motivo

Sin verdad de referencia objetiva, «90 % de acierto» no es verificable. Los bits de razón de Intel la dan gratis en los equipos que los exponen, y la degradación controlada mide exactamente lo que interesa: cuánto se equivoca el producto en los equipos que no los exponen.

## Riesgos abiertos

| Riesgo | Impacto | Mitigación/decisión requerida |
|---|---:|---|
| Frecuencia activa por PDH poco fiable en algún equipo | Medio | contraste con APERF/MPERF en el spike; reloj LHM como `substitute` con confianza baja |
| Nivel A inalcanzable sin UAC recurrente | **Decisivo** | puerta de viabilidad en fase 0 (`plan.md`); alternativas: servicio privilegiado revisado o reposicionar el producto en niveles B/C |
| Proveedor de acceso no permite limpiar bits de registro | Medio | usar bits instantáneos con confianza reducida un nivel |
| AMD sin razones documentadas | Alto | tabla PM del SMU solo con versiones permitidas; si no, nivel B como máximo |
| Relación potencia-frecuencia cúbica inexacta | Medio | rango `[0,5·g, 1,0·g]`, acotación por la frecuencia de turbo y contraste con la medición guiada |
| Gestión de energía de Windows (EcoQoS, EPP) confundida con limitación | Medio | sin meseta ni razón → `indeterminate` con esa causa alternativa, nunca térmica |
| Clasificación P/E/LP no disponible en LHM | Medio | CPUID hoja 0x1A / `GetLogicalProcessorInformationEx` en el sidecar; `unknown` degrada a homogéneo con aviso |
| Redistribución/instalación de acceso bajo nivel | Alto | revisión de licencia, firma y spike de instalador |
| Diferenciar límite térmico y potencia sin razones | Alto | mesetas y ventana de turbo; techo de confianza media en B y baja en C; SC-016 mide el error |
| Prueba integrada genera carga no representativa | Medio/alto | perfilar carga y ofrecer modo observación externa |
| SVG con demasiados puntos o segmentos eleva CPU/memoria | Medio | 2.000–3.000 puntos por pista, agregación Rust, ventanas por zoom y benchmark en hardware objetivo |
| Sensores AMD con semánticas Tctl/Tdie distintas | Medio | normalizador conserva semántica y elige representante con reglas probadas |
| Antivirus/SmartScreen ante sidecar nuevo | Medio | firma de código, reputación y paquete MSIX/instalador evaluado |
| Clave de firma del actualizador comprometida | Alto | secreto solo en CI protegida, rotación documentada y clave pública fijada en binario |
| Restauración de ventana con topología de monitores distinta | Bajo/medio | validar área visible y recentrar antes de mostrar |
| Traducciones divergentes o literales sin catálogo | Medio | igualdad de claves y detector de literales en CI |
