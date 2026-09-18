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
- **Hipótesis de trabajo (a confirmar en T019):** `LibreHardwareMonitorLib` no expone reloj efectivo (solo reloj por núcleo multiplicador × bus), ni banderas PROCHOT/PL1/PL2/EDP como sensores, ni la clase de núcleo P/E/LP. Decisiones derivadas: reloj efectivo derivado del contador PDH en Rust; banderas leídas del MSR por el sidecar cuando el acceso de bajo nivel lo permite; clasificación de núcleos por CPUID/`GetLogicalProcessorInformationEx` en el sidecar. Si el spike contradice la hipótesis, se simplifica; si la confirma y el MSR no es accesible, `thermal_confirmed` queda limitado a equipos con driver instalado y la cobertura lo dice.
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

### Decisión

No usar frecuencia turbo máxima ni una puntuación pública como denominador. Usar baseline local del mismo equipo y carga comparable; denominar el resultado “rendimiento disponible estimado”.

### Razón

El turbo máximo suele aplicar a pocos núcleos y condiciones transitorias. Un portátil y un sobremesa con la misma CPU tienen límites sostenidos diferentes. El reloj efectivo de núcleos inactivos tampoco representa capacidad perdida.

### Limitación conocida

La razón de relojes no equivale exactamente a razón de rendimiento de toda aplicación. El MVP comunica un rango y la base de la estimación. Una versión futura podría añadir microbenchmark calibrado y contadores IPC, siempre como método separado.

## 6. Detección térmica

### Decisión

Motor basado en reglas explicables, no ML, con señales directas, correlación temporal y factores de confusión.

### Motivos

- Determinista, auditable y comprobable con trazas.
- El corpus inicial no justificaría un modelo estadístico entrenado.
- Permite explicar “por qué” y revisar umbrales.

### Regla de comunicación

- Señal directa fiable → “confirmada”.
- Patrón correlacionado fuerte → “probable/compatible”.
- Solo calor → “temperatura alta; pérdida no demostrada”.
- Evidencia eléctrica dominante → “limitada por potencia”.

## 7. Baseline

### Decisión

Mantener referencias por grupos de núcleos, bucket de carga, alimentación/perfil y origen del reloj. Invalidarlas ante cambio de CPU/firmware significativo, versión incompatible del normalizador o calidad insuficiente.

### Construcción

- **Explícita:** prueba guiada o sesión marcada por el usuario.
- **Aprendida:** ventanas históricas estables con alto margen térmico y carga suficiente.
- **Comparativa:** antes/después de una intervención usando la misma versión y protocolo.

Nunca sustituir un baseline ausente por el turbo comercial.

## 8. Prueba de carga

### Decisión pendiente de spike

Preferencia inicial: carga integrada simple, progresiva y cancelable, únicamente si puede aislarse y detenerse con garantías. Alternativa: modo guiado que observa una carga externa del usuario.

### Condiciones para aprobar carga integrada

- Watchdog fuera de los workers.
- Cancelación en menos de 500 ms.
- Parada ante pérdida del proceso padre, sensor crítico o condición de seguridad.
- Sin instrucciones AVX extremas como única carga; perfiles documentados.
- Mensaje explícito de que no es un benchmark homologado.

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
- reloj efectivo o sustituto;
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

## Riesgos abiertos

| Riesgo | Impacto | Mitigación/decisión requerida |
|---|---:|---|
| Reloj efectivo no disponible ampliamente | Alto | derivado por contador PDH (`derived`), reloj LHM como `substitute`; porcentaje permitido con confianza ≤ `medium` |
| Banderas térmicas/eléctricas no expuestas por LHM | Alto | lectura MSR en el sidecar vía acceso de bajo nivel; sin ella, techo `thermal_probable` y aviso en cobertura |
| Clasificación P/E/LP no disponible en LHM | Medio | CPUID hoja 0x1A / `GetLogicalProcessorInformationEx` en el sidecar; `unknown` degrada a homogéneo con aviso |
| Redistribución/instalación de acceso bajo nivel | Alto | revisión de licencia, firma y spike de instalador |
| Diferenciar límite térmico y potencia sin banderas | Alto | clase indeterminada; no forzar causa |
| Prueba integrada genera carga no representativa | Medio/alto | perfilar carga y ofrecer modo observación externa |
| SVG con demasiados puntos o segmentos eleva CPU/memoria | Medio | 2.000–3.000 puntos por pista, agregación Rust, ventanas por zoom y benchmark en hardware objetivo |
| Sensores AMD con semánticas Tctl/Tdie distintas | Medio | normalizador conserva semántica y elige representante con reglas probadas |
| Antivirus/SmartScreen ante sidecar nuevo | Medio | firma de código, reputación y paquete MSIX/instalador evaluado |
| Clave de firma del actualizador comprometida | Alto | secreto solo en CI protegida, rotación documentada y clave pública fijada en binario |
| Restauración de ventana con topología de monitores distinta | Bajo/medio | validar área visible y recentrar antes de mostrar |
| Traducciones divergentes o literales sin catálogo | Medio | igualdad de claves y detector de literales en CI |
