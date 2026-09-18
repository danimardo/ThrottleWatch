# Plan de implementación: diagnóstico térmico de CPU

**Rama**: `001-cpu-thermal-diagnostics` | **Fecha**: 2026-09-17 | **Spec**: `spec.md`

## Resumen

Construir ThrottleWatch, una aplicación de escritorio Windows local y visual que reciba sensores de CPU desde un colector .NET basado en LibreHardwareMonitor, normalice las diferencias Intel/AMD, conserve series temporales en SQLite y ejecute un motor determinista que distinga limitación térmica, eléctrica, mixta o indeterminada. Tauri aloja una interfaz Svelte/TypeScript bilingüe con onboarding, temas y barra de título propia; el backend Rust controla ciclo de vida, persistencia, seguridad IPC, diagnóstico, exportación y actualizaciones voluntarias firmadas.

## Contexto técnico

**Lenguajes/versiones**: Rust estable; TypeScript estricto; Svelte 5; .NET 10 LTS/C#; SQL  
**Dependencias principales**: Tauri 2.x y plugins oficiales necesarios para notificaciones, inicio automático y actualizaciones, Svelte + Vite, LibreHardwareMonitorLib, SQLite (`rusqlite` o equivalente Rust), JSON Schema. El gráfico de análisis es el componente SVG propio `AnalysisChart`; no se incorpora una biblioteca de gráficos.  
**Almacenamiento**: SQLite local con WAL, migraciones versionadas y exportación CSV/JSON  
**Pruebas**: `cargo test`, Vitest, Testing Library, Playwright para UI, xUnit para colector, reproducción de trazas para integración  
**Plataforma objetivo**: Windows 10/11 de 64 bits; x64 primero, ARM64 solo tras validar dependencias  
**Tipo de proyecto**: aplicación de escritorio con sidecar local  
**Objetivos de rendimiento**: UI fluida; muestra visible <1,5 s p95; monitorización pasiva <1 % CPU promedio; memoria total objetivo <180 MB  
**Restricciones**: funcionamiento offline completo; red del actualizador solo por opt-in; UI no elevada; sensores variables; ausencia de datos válida; no cambiar parámetros del hardware  
**Escala**: un equipo y una CPU; 1 muestra/s; 7 días de datos brutos por defecto; sesiones importables

> Las versiones menores/parches se fijarán al crear el repositorio y se actualizarán mediante PR controlada. No se deben usar versiones flotantes en CI.

## Comprobación de la constitución

| Puerta | Decisión de diseño | Estado |
|---|---|---|
| Evidencia antes que afirmación | Clasificación por niveles de cobertura + confianza con techo + evidencias; cifras solo por techo de potencia o medición guiada | Cumple |
| Degradación elegante | Modelo de capacidades y valores `null` con calidad | Cumple |
| Causalidad prudente | Motor térmico y motor eléctrico independientes; admite causa mixta | Cumple |
| Privilegio mínimo | UI sin elevar; instalación del acceso de bajo nivel separada | Cumple |
| Local y privado | SQLite local; cero red requerida; actualizador apagado por defecto; anonimización | Cumple |
| Visualización fiel | huecos reales, escalas etiquetadas, color + texto + icono | Cumple |
| Reproducibilidad | motor puro y reproducción de trazas | Cumple |
| Seguridad térmica | carga voluntaria, cancelable y con watchdog | Cumple |

Revisar de nuevo tras completar los spikes de acceso a sensores y prueba guiada.

## Arquitectura

```mermaid
flowchart TD
    UI["Svelte UI\nPanel y análisis"] -->|comandos Tauri| CORE["Backend Rust\nEstado, SQLite y diagnóstico"]
    CORE -->|stdio NDJSON + nonce| AGENT["Sidecar .NET\nNormalizador de sensores"]
    AGENT --> LHM["LibreHardwareMonitorLib"]
    LHM --> HW["CPU Intel/AMD\nSensores y acceso bajo nivel"]
    CORE --> DB["SQLite local"]
    CORE --> EXPORT["CSV / JSON anonimizable"]
    CORE -.->|solo opt-in, HTTPS y firma| RELEASES["GitHub Releases\nendpoint fijo"]
```

### Responsabilidades

**Frontend Svelte**

- Composición visual, onboarding, navegación, localización, temas, barra propia, accesibilidad y preferencias de presentación.
- Suscripción a un modelo ya normalizado; no interpreta nombres de sensores.
- Renderizado de tarjetas, series, mapa de núcleos, evidencias e informes.
- Integración del sistema canónico de `design/`: tokens, componentes, contratos y composiciones de referencia. Las adaptaciones a datos reales se realizan mediante propiedades y adaptadores, no duplicando estilos o componentes dentro de cada feature.

**Backend Tauri/Rust**

- Inicia, supervisa y detiene el sidecar.
- Valida cada mensaje contra contrato y aplica límites de tamaño/frecuencia.
- Mantiene reloj monotónico, agrupa muestras y persiste lotes.
- Ejecuta el motor de diagnóstico puro y versionado.
- Expone comandos de lectura, sesión, exportación, configuración, ciclo de vida y actualización.
- Centraliza las API de ventana; ningún componente de pantalla llama directamente a Tauri.
- Lee el contexto energético (fuente, plan, batería) mediante Win32 y lo adjunta a cada frame; escucha `WM_POWERBROADCAST` para detectar suspensión/reanudación y cambios de alimentación.
- Calcula la frecuencia activa y la frecuencia base por procesador lógico con los contadores PDH `% Processor Performance` y `Processor Frequency`, y las registra como descriptores de origen `host` con calidad `derived`.
- Gestiona los límites de sesión pasiva (inicio, hueco > 60 s, 24 h) y congela informes.
- Aplica la instancia única, la bandeja (icono de cinco estados y menú), las notificaciones y sus reglas de persistencia/enfriamiento.
- Resuelve la geometría antes de mostrar la ventana y valida que quede dentro de monitores activos.
- Ejecuta comprobación y descarga fuera de la WebView y verifica Ed25519 antes de ofrecer instalar.
- No accede directamente a MSR/SMU en el MVP.

**Sidecar .NET**

- Integra LibreHardwareMonitorLib sin ejecutar su GUI.
- Descubre hardware y sensores; conserva nombre/ID original.
- Normaliza magnitud, unidad, alcance y topología cuando puede.
- Emite capacidades y muestras; no emite diagnósticos de negocio.
- Acepta una lista mínima de comandos: `hello`, `start`, `set_rate`, `snapshot`, `stop`, `shutdown`.
- Clasifica los grupos P/E/LP con `GetLogicalProcessorInformationEx` y CPUID; LibreHardwareMonitorLib no lo expone.
- Intenta leer las banderas térmicas/eléctricas del MSR de estado (Intel `IA32_THERM_STATUS`, `IA32_PACKAGE_THERM_STATUS`, `MSR_CORE_PERF_LIMIT_REASONS`; equivalentes AMD) a través del acceso de bajo nivel disponible; sin acceso, no emite esos descriptores.
- Termina al recibir EOF en `stdin` o cuando desaparece el PID padre (comprobación cada 2 s).

**Acceso de bajo nivel**

- La instalación de un controlador firmado, si la distribución lo permite y el usuario lo acepta, ocurre en un paso explícito con UAC.
- La aplicación intenta primero modo estándar. La cobertura reducida es un estado soportado.
- El MVP no elevará silenciosamente la UI ni ejecutará comandos arbitrarios.
- Un servicio privilegiado persistente solo se añadirá si un spike demuestra que es imprescindible y supera una revisión de amenazas.

## Flujo de datos

1. Rust genera un nonce efímero e inicia el sidecar desde una ruta empaquetada.
2. Ambos negocian versión de protocolo y capacidades.
3. El sidecar envía catálogo de sensores y después una muestra normalizada por segundo.
4. Rust valida, marca calidad/lag, almacena en búfer y publica un snapshot agregado a la UI.
5. Cada ventana temporal actualiza eventos y diagnóstico.
6. Las muestras se escriben por lotes; la UI nunca espera a disco.
7. Al finalizar una sesión se congela un informe con versión de reglas, nivel de cobertura y, si es guiada, su resultado medido.

### Resolución temporal para análisis

- `AnalysisChart` dibuja hasta cuatro pistas sincronizadas mediante SVG y recibe puntos ya preparados; no consulta SQLite ni reduce datos en la UI.
- Una vista de sesión debería entregar aproximadamente 2.000–3.000 puntos como máximo por pista y 10.000–12.000 en total. Estos son límites de legibilidad y presupuesto objetivo, no una autorización para truncar datos silenciosamente.
- Para rangos que excedan el presupuesto, Rust agrega por cubos temporales ajustados a la resolución solicitada y conserva, como mínimo, primer/último valor válido, mínimo, máximo y promedio cuando correspondan.
- Un cubo que contiene un hueco no une segmentos a través de él. Debe emitir una discontinuidad explícita o dividir el tramo.
- La calidad agregada es la peor calidad material del cubo y conserva la distinción entre dato reducido por resolución y lectura originalmente degradada; `reducedQuality` no se reutiliza para significar downsampling.
- Los límites de eventos y cambios de clasificación se preservan como puntos/bordes obligatorios aunque no coincidan con un cubo.
- El zoom solicita a Rust una ventana de mayor resolución en lugar de estirar indefinidamente la serie agregada.

## Motor de diagnóstico

Revisado el 2026-09-18 (véase `gap-analysis.md` § 12). Los valores concretos están en `spec.md` § «Parámetros iniciales»; aquí se describe la estructura.

### Entradas

- Por procesador lógico: utilidad, frecuencia activa (`% Processor Performance` × `Processor Frequency`) y frecuencia base (`Processor Frequency`), leídas por Rust vía PDH.
- Temperatura representativa y **límite térmico efectivo** (TjMax − TCC offset, o tabla por familia en AMD).
- Potencia de paquete; en nivel A, límite de potencia efectivo (PL1/PL2/Tau) y razones de limitación como bits de registro.
- Contexto: alimentación, plan energético, fase de sesión y marca de ventana de turbo.

### Etapas

1. **Nivel de cobertura** (A/B/C) a partir del catálogo de capacidades; fija qué reglas pueden aplicarse y el techo de confianza.
2. **Núcleos activos**: utilidad ≥ 80 %; la carga sostenida y la frecuencia activa se calculan solo sobre ellos, por grupo P/E/LP.
3. **Ventana de turbo**: detecta inicios de carga y excluye `max(Tau, 60 s)`; emite `turbo_end` al ver el escalón de potencia.
4. **Rasgos de la ventana estable** (60 s, deslizante cada 10 s): ocupación de cada razón, meseta térmica, meseta de potencia, tendencia del límite o del nivel de meseta de potencia a lo largo de la sesión, razón frecuencia activa / base.
5. **Clasificación** por la tabla ordenada de `spec.md` (primera regla que se cumple), con subtipo de equipo y gravedad `boost`/`below_base`.
6. **Confianza**: puntuación de solidez con techo por nivel.
7. **Potencial** (solo nivel A y clases térmicas, mixta o chasis): método «techo de potencia» `g = (PL1 / P)^(1/3) − 1`, acotado por la frecuencia observada en la ventana de turbo, expresado como `[0,5·g, 1,0·g]` y redondeado a tramos.
8. **Eventos**: fusión de ventanas consecutivas de la misma clase en `limit_event` con evidencias como códigos.

### Por qué así

- **Mesetas en vez de caídas**: la limitación es un *estado* (una magnitud clavada en su tope), no un cambio. Así se detecta también el equipo que ya empieza caliente o que se degrada poco a poco, y no se confunde el fin del turbo con calor.
- **Frecuencia base como umbral de gravedad**: es la única frecuencia que el fabricante garantiza; por encima, la limitación térmica es el funcionamiento previsto del boost.
- **Techo de potencia en vez de baseline**: responde directamente a «¿cuánto ganaría enfriando mejor?» con una relación física, sin depender del tipo de carga ni de la calidad de una referencia aprendida.
- **Medición directa en la prueba guiada**: el generador cuenta trabajo; es la única cifra de rendimiento que no es una inferencia.

### Clasificaciones

- `normal`
- `hot_unproven`
- `thermal_probable`
- `thermal_confirmed`
- `power_limited`
- `platform_limited` (subtipos `chassis_thermal`, `external_prochot`)
- `mixed_limit`
- `indeterminate`

Atributo transversal: `limit_severity` = `boost` | `below_base`.

Las reglas, umbrales y la tabla `thermal-limits-v1` se almacenan en configuración versionada, no dispersos por la UI.

## Prueba guiada

Fases (valores en `spec.md` § Parámetros iniciales):

1. Comprobación de sensores y contexto (≤ 30 s).
2. Reposo opcional para estabilizar (60 s, omitible).
3. Calentamiento progresivo (90 s).
4. Carga sostenida con muestreo de diagnóstico (180/240/360 s según `guided.duration`); se analizan los últimos 120 s, siempre fuera de la ventana de turbo.
5. Recuperación (120 s).
6. Resultado e informe, con rendimiento medido y oferta de marcar como referencia.

El generador cuenta operaciones completadas por hilo y segundo (bucle de trabajo fijo y versionado, sin instrucciones AVX-512 y con AVX2 solo en un perfil documentado) y lo publica en cada muestra. Ese contador es la medida de rendimiento de la prueba.

Parada automática (FR-085): temperatura > límite efectivo + 2 °C durante 3 muestras, temperatura en el límite con frecuencia activa < 50 % de la base durante 10 s, 3 muestras consecutivas sin sensor crítico o 5 s sin latido del generador. Alcanzar el límite térmico no detiene la prueba: es lo que se mide y el propio procesador se protege. Ocultar la ventana o suspender el equipo cancela la prueba.

El generador de carga debe estar aislado del hilo de sensores, permitir cancelación inmediata, tener watchdog y detenerse ante pérdida del sensor crítico, proceso padre ausente o umbral de seguridad. Antes de implementar carga propia se realizará un spike para decidir entre carga integrada y guía sobre una carga externa reproducible.

## Almacenamiento y retención

- SQLite en el directorio de datos de aplicación, WAL y `busy_timeout`.
- Escritura de muestras en lotes pequeños transaccionales.
- Datos brutos según retención del usuario; por defecto siete días.
- Informes y resultados guiados se conservan hasta borrado explícito.
- Mantenimiento en segundo plano con límite de tiempo; nunca durante la fase estable de una prueba.
- Exportación desde una instantánea transaccional para evitar ficheros incoherentes.
- Preferencias tipadas, onboarding, geometría y actualización en tablas versionadas de volumen constante.
- Borrado de datos y restablecimiento total son transacciones distintas; el primero conserva preferencias.
- Ruta de datos `%LOCALAPPDATA%\ThrottleWatch`; registros en `logs/` con rotación 5 × 5 MB.
- Disco lleno: modo solo memoria, aviso persistente y reintento cada 60 s. Base de datos dañada: renombrado a `.corrupt-<fecha>`, base nueva y oferta de exportar la dañada.
- Persistencia por núcleo solo en perfil `diagnostic`, sesiones guiadas o `sampling.per_core_history`; el resto se agrega por grupo antes de escribir.

## Primer inicio, localización y ventana

- El shell arranca oculto, aplica tema e idioma, restaura geometría válida y después muestra la ventana para evitar destellos o saltos.
- El router decide entre onboarding pendiente/reanudado y aplicación principal; omitir inicia o conserva la detección pasiva.
- Los catálogos `es` y `en` se validan en CI con igualdad de claves y detección de literales visibles.
- La resolución de locale es una función pura probada con `es`, `ca`, `gl`, `eu`, `ast`, `an`, `pt` y fallbacks desconocidos.
- El componente de barra es presentacional; un adaptador único encapsula minimizar, maximizar/restaurar, cerrar y eventos de tamaño.
- La acción de primera X se decide en Rust para cubrir también cierre nativo; `unset` abre el diálogo y una elección persistida decide cierres posteriores. Elegir bandeja activa `tray.monitoring_enabled`; `Esc` no persiste.
- Ventana mínima 480×600 y inicial 1100×760; instancia única mediante el plugin oficial.
- Números y fechas con `Intl` según el idioma efectivo; solo °C.
- El shell aplica `data-theme`, `data-motion` y `data-glass` en `<html>` (helpers `applyGlassLevel`/`applyMotionLevel` de `tokens.ts`) y monta la clase `tw-ambient` en el contenedor raíz; los cambios de «Efectos de transparencia» y «Reducir movimiento» de Windows se escuchan en caliente igual que el tema.

## Actualizaciones

- Endpoint de manifiesto fijo bajo GitHub Releases del proyecto, no configurable por UI ni argumentos.
- Interruptor apagado de fábrica; el backend no crea peticiones mientras sea `false`.
- Planificador automático con cadencia mínima de 24 h y comando manual separado que puede reintentar antes.
- Máquina de estados `idle → available → downloading → verified → installing`, con errores recuperables y descarte de parciales.
- Verificación Ed25519 durante la descarga antes de alcanzar `verified`; la canalización de release genera manifiesto y firmas.
- Instalar comprueba bloqueos de diagnóstico/exportación/importación/borrado, pide confirmación y lanza el instalador tras cierre ordenado.
- Sin canal de actualización; estados expuestos a la UI: `idle | checking | up_to_date | available | downloading | verified | installing | error`.

## Seguridad y privacidad

- Sidecar empaquetado, hash esperado y ruta fija; no se aceptan ejecutables configurables.
- Nonce de sesión por proceso; mensajes con versión, secuencia y tamaño máximo.
- Validación estricta de JSON; cierre del canal tras errores repetidos.
- Comandos Tauri con lista permitida y parámetros tipados.
- CSP restrictiva; sin contenido web remoto en la WebView.
- Logs rotados y depuración opt-in.
- Exportación anónima elimina los campos definidos en contrato y vuelve a validar el resultado.
- SBOM y avisos de MPL 2.0 y dependencias en el instalador y `Acerca de`.

## Observabilidad

- Logs JSON por componente con `session_id`, `protocol_version`, severidad y código de error.
- Métricas locales de latencia de muestra, muestras descartadas, cola de persistencia y tiempo de render.
- Pantalla de diagnóstico técnico capaz de copiar un resumen sin identificadores.
- Nunca registrar cada muestra en nivel normal para evitar E/S y crecimiento excesivo.

## Estrategia de pruebas

### Unitarias

- Normalización de sensores y unidades.
- Agregación por topología.
- Reglas de clasificación y confianza: una prueba por fila de la tabla y por orden de precedencia.
- Detección de ventana de turbo, mesetas, núcleos activos y gravedad frente a la frecuencia base.
- Potencial por techo de potencia (acotación, tramos, redondeo hacia fuera) y ausencia de cifra sin PL1.
- Comparabilidad de resultados guiados y anonimización.

### Contrato

- JSON Schema para todos los mensajes.
- Compatibilidad de protocolo y rechazo de mensajes malformados/excesivos.
- Fixtures C# ↔ Rust para evitar divergencias de serialización.

### Integración

- Sidecar falso que reproduce trazas Intel, AMD, legado, híbrido y degradado.
- **Validación del motor con corpus etiquetado** (`research.md` § 15): trazas de nivel A etiquetadas con los bits de razón como verdad de referencia, y sus copias degradadas a B y C para medir SC-003, SC-004, SC-005 y SC-016 a SC-018 en CI.
- Caída/reinicio del sidecar, lag, secuencias perdidas y reanudación del sistema.
- SQLite: migraciones, retención, disco lleno simulado y exportación consistente.

### UI y accesibilidad

- Estados vacío, normal, caliente, térmico (`boost` y `below_base`), potencia, equipo (chasis y señal externa), mixto, fin de turbo y desconectado; niveles de cobertura A/B/C.
- Onboarding completo, omitido, reanudado y novedades versionadas.
- Matriz de locale, igualdad de catálogos y expansión de texto español/inglés.
- Tema sistema/claro/oscuro, cambio de Windows en caliente y movimiento reducido.
- Barra propia, geometría fuera de pantalla y pregunta de primera X.
- Gráfico SVG: cursor y selección por teclado, eventos accionables, resumen textual y tabla accesible del rango.
- Navegación completa por teclado, foco, lector de pantalla y movimiento reducido.
- Pruebas visuales a 100 %, 125 %, 150 % y 200 % de escala de Windows.

### Hardware real

- Matriz mínima: Intel híbrido moderno, Intel anterior, AMD Ryzen moderno, AMD anterior y equipo con cobertura degradada.
- Comparar mensajes con señales expuestas por herramientas de referencia, sin exigir igualdad exacta de muestreo.

## Estructura del proyecto

```text
apps/
├── desktop/
│   ├── src/                    # Svelte/TypeScript
│   │   ├── components/
│   │   ├── features/
│   │   ├── routes/
│   │   ├── stores/
│   │   └── styles/
│   ├── src-tauri/              # Rust/Tauri
│   │   └── src/
│   │       ├── commands/
│   │       ├── diagnostics/
│   │       ├── ipc/
│   │       ├── storage/
│   │       └── export/
│   └── tests/
└── sensor-agent/               # .NET sidecar
    ├── Collector/
    ├── Normalization/
    ├── Protocol/
    └── Tests/
design/                         # fuente canónica de UI, tokens, ejemplos y harness
packages/
├── contracts/                  # schemas y fixtures compartidos
└── trace-fixtures/
scripts/
specs/
tests/
├── integration/
├── replay/
└── hardware/
```

**Decisión estructural:** monorepo con tres límites explícitos: UI, núcleo Tauri y colector. Los contratos y trazas son compartidos, pero la lógica de diagnóstico pertenece únicamente a Rust.

### Integración del sistema de diseño

- `design/tokens/tokens.css` alimenta los estilos de la aplicación; no se mantiene una copia divergente de los tokens.
- **Decisión (2026-09-18):** `design/` se **copia** a `apps/desktop/src/design-system/` mediante un script de sincronización (`pnpm design:sync`) y un test de CI comprueba la igualdad byte a byte de `components/`, `icons/`, `illustrations/`, `lib/` y `tokens/`. `design/` sigue siendo la única fuente editable; la app nunca modifica su copia. `design/examples/` y `design/harness/` no se copian ni se importan. Motivo: el harness fija Svelte 5.57 y Tauri fijará su propia versión; un paquete workspace contradice la decisión de `design/README.md` de no publicar paquete.
- `design/examples/` define la composición de referencia, mientras que las features conectan datos y comandos mediante adaptadores Svelte/Tauri.
- `design/harness/` es la prueba aislada del sistema: sus comandos de comprobación y compilación forman parte de CI.
- Cada pantalla se valida en español e inglés, claro y oscuro y tres niveles de ancho: compacto, medio y expandido. Cualquier excepción queda registrada en este plan o en un ADR.

## Fases

### Fase 0 — Riesgos y spikes

**Puerta de viabilidad del producto.** Antes de la fase 1 se decide si el nivel A es alcanzable en condiciones aceptables para un usuario corriente:

1. Con el proveedor de acceso de bajo nivel instalado una vez (PawnIO o equivalente firmado), ¿puede el sidecar leer `0x64F`, `0x1A2`, `0x610` y limpiar los bits de registro **sin pedir UAC en cada arranque**?
2. ¿Qué porcentaje de la matriz de hardware objetivo alcanza nivel A (Intel) y nivel B (con potencia) sin acceso?
3. En AMD, ¿qué versiones de la tabla PM del SMU se pueden incluir en la lista permitida?

Resultados posibles: **(a)** nivel A sin UAC recurrente → se continúa con el plan completo. **(b)** Nivel A solo con un servicio privilegiado → se continúa si el servicio supera la revisión de amenazas exigida por la constitución (principio IV); si no, (c). **(c)** Nivel A inalcanzable → el producto se reposiciona como explicador prudente (niveles B/C), se retiran la clasificación confirmada y el potencial cuantificado de la propuesta de valor y se decide explícitamente si continuar. La decisión se registra en un ADR.

- Confirmar sensores expuestos en Intel/AMD y comportamiento sin privilegios.
- Probar empaquetado del sidecar en x64 y validar firma/controlador.
- Validar IPC, latencia y reinicio.
- Verificar que `% Processor Performance` y `Processor Frequency` son fiables por procesador lógico en Intel híbrido y AMD (comparando con APERF/MPERF leídos por el sidecar cuando haya acceso), incluidos equipos con EcoQoS y modos de eficiencia.
- Grabar el corpus etiquetado inicial (véase `research.md` § 15) en al menos un Intel híbrido, un Intel anterior, un portátil con gestión térmica del fabricante y un AMD Zen 4.
- Validar en el hardware objetivo el presupuesto del `AnalysisChart` SVG con 4 pistas, 3.000 puntos por pista y eventos superpuestos; conservar el benchmark reproducible.
- Medir en WebView2 el coste del material de vidrio (`backdrop-filter` en tarjetas y chrome, fondo ambiental animado) en reposo y con el gráfico actualizándose; fijar el umbral de degradación automática a `reduced` (propuesta: < 50 fps sostenidos durante 3 s o > 1 % de CPU en reposo atribuible a composición) y leer `UISettings.AdvancedEffectsEnabled` para el modo `system`.
- Decidir estrategia segura de carga guiada.
- Validar permisos mínimos de barra propia, inicio con Windows, notificaciones y actualizador.
- Definir pipeline de GitHub Releases, clave pública Ed25519 y custodia del secreto de firma.

### Fase 1 — Vertical slice pasivo

Resolver idioma/tema → recorrer u omitir onboarding → descubrir CPU → emitir muestras → validar en Rust → mostrar cuatro métricas → almacenar una sesión → reproducirla.

### Fase 2 — Diagnóstico y visualización

Normalización completa, ventana de turbo, mesetas, eventos, clasificación con gravedad, potencial por techo de potencia, gráficos sincronizados, mapa de núcleos e informe.

### Fase 3 — Operación de escritorio

Bandeja, primera acción de cierre, geometría, inicio con Windows, ajustes, alertas, retención, exportación/importación, recuperación del sidecar, instalador y actualizaciones firmadas.

### Fase 4 — Prueba guiada y endurecimiento

Carga segura si el spike la aprueba, matriz de hardware, accesibilidad, rendimiento, firma y documentación.

## Seguimiento de complejidad

| Complejidad | Por qué es necesaria | Alternativa descartada |
|---|---|---|
| Sidecar .NET además de Rust | LibreHardwareMonitorLib es .NET y evita reimplementar familias de CPU | Portar acceso MSR/SMU a Rust aumenta riesgo y reduce cobertura |
| Tres niveles de evidencia | Los sensores difieren y la causalidad no siempre es directa | Un umbral de temperatura produciría falsos positivos |
| Resultado guiado medido como única referencia | Carga idéntica por construcción; permite comparar «antes/después» con rendimiento real | Tabla global o baselines aprendidos: no capturan firmware, chasis ni tipo de carga |
| Barra de título propia | Identidad visual coherente y comportamiento aprobado por el usuario | La decoración nativa no permite la experiencia solicitada |
| Actualizador firmado opcional | Entrega correcciones sin renunciar a control ni funcionamiento offline | Una actualización silenciosa viola privacidad y consentimiento |
| Gráfico SVG propio | Cumple huecos, calidad, eventos y teclado con una API Svelte controlada y sin dependencia de gráficos | ECharts duplicaría interacción/ARIA y ampliaría tamaño/superficie; reconsiderar solo si el benchmark real incumple el presupuesto |
| Frecuencia activa por contador PDH en Rust | Es la frecuencia mientras el núcleo ejecuta (APERF/MPERF de Windows) y está disponible sin driver; es la métrica correcta para detectar limitación | El reloj nominal de LHM no refleja la frecuencia real; el «effective clock» con reposo mezcla carga y frecuencia |
| Lectura MSR de razones, límites y TCC offset en el sidecar | Sin razones directas no hay confirmación ni atribución fiable, y sin TCC offset el margen es falso en muchos portátiles | Inferir «confirmada» desde el margen es una inferencia disfrazada de confirmación |
| Clasificación por mesetas y ventana de turbo | Evita el falso positivo PL2→PL1 y detecta estados, no solo cambios | La regla anterior (caída de reloj tras calor) confundía el fin del turbo con limitación térmica |
| Potencial por techo de potencia y medición guiada | Fundamento físico o medido, independiente del tipo de carga | Baselines aprendidos: sesgados hacia momentos de poca exigencia y dependientes del tipo de carga |
| Contexto energético en Rust | El motor necesita batería/plan como factor de confusión y el sidecar no debe conocer Win32 de energía | Emitirlo como sensores mezcla contexto con hardware y amplía el contrato privilegiado |
| Copia sincronizada del sistema de diseño | Aísla versiones de Svelte entre harness y app manteniendo una única fuente | Importar por ruta relativa acopla versiones; paquete workspace contradice `design/README.md` |
