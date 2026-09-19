# Spike T019c: coste del vidrio en WebView2

**Estado:** medido en el Ryzen 5 2600X (cota inferior); **pendiente el equipo de referencia**
(Intel híbrido 12.ª gen o posterior, `plan.md` § Matriz), que no está disponible.
**Fecha:** 2026-09-19

## Banco de medida

`docs/spikes/glass-bench/` (no forma parte de la aplicación):

- `src/Bench.svelte` monta el shell tal como lo prescribe el sistema de diseño (fondo
  `.tw-ambient`, barra `tw-glass-strong`, barra lateral `tw-glass`, tarjeta `tw-glass` con el
  `AnalysisChart` real de `design/components/` y un panel `tw-glass-strong` flotante sobre el
  gráfico) con 4 pistas × 3.000 puntos y una muestra nueva por intervalo.
  Parámetros: `glass=full|reduced|off`, `interval=ms` (0 = reposo), `points`, `seconds`,
  `delay`, `ambient=0` (fondo estático), `chart=0`, `measure=0` (sin contador `requestAnimationFrame`).
- `host/` es un anfitrión WebView2 mínimo (.NET 10 + `Microsoft.Web.WebView2`): carga la URL,
  espera 3 s, y durante la ventana suma el tiempo de CPU de **todos** los procesos de esa
  instancia de WebView2 (navegador, GPU, renderizador, utilidades: 6 procesos), no del
  anfitrión. Escribe un JSON por ejecución.

```text
cd docs/spikes/glass-bench && npm install && npm run build && npm run preview   # puerto 4179
cd host && dotnet build -c Release
bin/Release/net10.0-windows/GlassHost.exe "http://localhost:4179/?glass=full&interval=1000&ambient=0" 15 out.json
```

**Equipo:** AMD Ryzen 5 2600X (12 procesadores lógicos), NVIDIA GeForce RTX 4060 Ti
(controlador 32.0.15.9186), 2560×1080 a 59 Hz, Windows 11 Pro 26200 con «Efectos de
transparencia» activados, WebView2 Evergreen **153.0.4234.32**, ventana 1100×760.
`% máquina` = tiempo de CPU / (ventana × 12); `% núcleo` = tiempo de CPU / ventana.

## Resultados

### 1. Shell completo tal como está en el sistema de diseño (fondo `.tw-ambient` animado)

| Vidrio | Actualización | fps mín/p10/mediana | CPU % máquina | CPU % núcleo |
|---|---|---|---|---|
| full | reposo | 60/60/60 | 2,76 | 33 |
| full | 1 s | 59/59/60 | 3,06 | 37 |
| full | 200 ms | 59/59/60 | 4,83 | 58 |
| reduced | reposo | 60/60/60 | 6,63 | 80 |
| reduced | 1 s | 60/60/60 | 7,01 | 84 |
| off | reposo | 60/60/60 | 7,06 | 85 |
| off | 1 s | 60/60/60 | 5,79 | 70 |
| off | 200 ms | 60/60/60 | 7,67 | 92 |

Reposo sin contador de fps (`measure=0`): full 2,46 % máquina (29 % núcleo); off 8,47 % máquina
(**102 % de un núcleo**).

### 2. Aislamiento del fondo animado (reposo, `measure=0`, 15 s)

| Configuración | CPU % máquina | CPU % núcleo |
|---|---|---|
| full, `.tw-ambient` animado | 2,46 | 29,5 |
| full, fondo estático | **0,05** | 0,6 |
| off, `.tw-ambient` animado | 8,47 | 101,7 |
| off, fondo estático | **0,05** | 0,6 |
| full, fondo estático, sin gráfico | 0,07 | 0,8 |
| off, fondo estático, sin gráfico | 0,01 | 0,1 |

### 3. Fondo estático, gráfico actualizándose

| Vidrio | Actualización | fps mín/p10/mediana | CPU % máquina | CPU % núcleo |
|---|---|---|---|---|
| full | 1 s | 60/60/60 | 0,80 | 9,6 |
| reduced | 1 s | 60/60/60 | 0,70 | 8,4 |
| off | 1 s | 60/60/60 | 0,62 | 7,4 |
| full | 200 ms | — | 3,09 | 37 |
| reduced | 200 ms | — | 2,96 | 36 |
| off | 200 ms | — | 2,68 | 32 |
| full | reposo, con contador rAF | 60/60/60 | 0,36 | 4,4 |

## Conclusiones

1. **El desenfoque no es el coste.** Con el fondo estático, el vidrio `full` añade ≈ 0,2 puntos
   de % máquina (≈ 2–5 % de un núcleo) sobre `off` mientras el gráfico se actualiza, y nada en
   reposo (0,05 % máquina en ambos). En este equipo, degradar de `full` a `reduced` u `off`
   apenas cambia el consumo.
2. **La animación `tw-ambient-drift` es el coste dominante**: 30 % de un núcleo en `full` y
   ~100 % de un núcleo en `off` en reposo, de forma continua, porque anima `background-position`
   de tres gradientes radiales con `background-attachment: fixed` a pantalla completa; en `off`
   el coste es aún mayor (hipótesis no verificada: sin desenfoque la capa deja de promoverse
   a la GPU y se rasteriza cada fotograma). Ese consumo supera
   30–100 veces `glass.degrade_idle_cpu_pct` (1 %) y haría degradar el vidrio nada más arrancar
   sin que el vidrio tenga la culpa. **Recomendación para el sistema de diseño** (fuera del
   alcance de este spike; se anota para `design/`): pausar `tw-ambient-drift` por defecto en la
   aplicación (o animarla solo con `transform` en una capa propia) y, en `off`, eliminar la
   animación además de las variables. Los umbrales `glass.*` no deben medirse con el fondo
   animado activo.
3. **Coste del gráfico**: 4 × 3.000 puntos SVG a 1 muestra/s cuestan 7–10 % de un núcleo; a
   200 ms, 32–37 %. Está dentro de lo previsto para el intervalo normal (1 s) y confirma que el
   intervalo de diagnóstico (200 ms) no debe redibujar el gráfico completo en cada muestra
   (agregar en Rust o redibujar por lotes, `plan.md` § riesgos «SVG con demasiados puntos»).
4. **fps**: 60 en todos los casos; el monitor va a 59–60 Hz y la GPU no es el cuello de botella.
   El umbral `glass.degrade_fps` (50 sostenidos 3 s) no se ha podido ejercitar aquí; solo el
   equipo de referencia o un equipo con GPU integrada podrá confirmarlo o corregirlo.
5. **El propio contador de fps cuesta** 0,36 % máquina (4,4 % de un núcleo) en reposo: un
   `requestAnimationFrame` continuo para vigilar `glass.degrade_fps` gasta más que el umbral
   de reposo que vigila. T131 debe medir fps solo en ventanas cortas (p. ej. 3 s tras cada
   cambio de nivel o de tamaño) o usar `PerformanceObserver`/eventos `long-animation-frame`,
   no un bucle permanente.

## Umbrales `glass.*` de `spec.md`

- `glass.degrade_idle_cpu_pct = 1`: **se mantiene** como valor, pero la especificación debe
  fijar la unidad: con estos datos, «1 % de la máquina» (12 núcleos) equivale a 12 % de un
  núcleo y se excede solo con el fondo animado; «1 % de un núcleo» quedaría al borde incluso
  sin vidrio (0,6 % en reposo con y sin desenfoque) y saltaría con cualquier ruido. Propuesta:
  % de la máquina, medido sobre los procesos de WebView2, con el fondo ambiental pausado.
- `glass.degrade_fps = 50` / `restore_fps = 55` y sus ventanas: **sin cambios**; no hay datos
  que los contradigan ni los confirmen en el equipo de referencia.
- Los valores siguen siendo provisionales (`spec.md` los marca así hasta este spike); el
  cambio de unidad exige nueva versión de reglas cuando se decida.

## Pendiente

- Repetir la matriz en el Intel híbrido de referencia (GPU integrada, Windows 11 25H2) y en un
  portátil con batería. El banco y el anfitrión están listos; basta ejecutar los comandos de
  arriba y pegar los JSON.
- La aplicación real aún no arranca como ventana Tauri (sin `src/main.rs` ni CLI de Tauri;
  véase `tauri-permissions.md`), por eso se midió en un anfitrión WebView2 propio con el mismo
  runtime Evergreen.
