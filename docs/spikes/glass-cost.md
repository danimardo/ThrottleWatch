# Spike T019c: coste del vidrio en WebView2

**Estado:** medido en el Ryzen 5 2600X (cota inferior) y en un Intel Core Ultra 7 155H
(Meteor Lake, equipo de referencia híbrido; ver § «Equipo de referencia Intel» más abajo).
**Fecha:** 2026-09-19 (AMD), 2026-09-22 (Intel)

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

### 2. Aislamiento del fondo animado (reposo, `measure=0`, 15 s; dos ejecuciones)

| Configuración | CPU % máquina (1.ª / 2.ª) | CPU % núcleo (1.ª / 2.ª) |
|---|---|---|
| full, `.tw-ambient` animado | 2,46 / 3,15 | 29,5 / 37,8 |
| full, fondo estático | **0,05 / 0,06** | 0,6 / 0,8 |
| off, `.tw-ambient` animado | 8,47 / 7,07 | 101,7 / 84,9 |
| off, fondo estático | **0,05 / 0,08** | 0,6 / 1,0 |
| full, fondo estático, sin gráfico | 0,07 / 0,11 | 0,8 / 1,4 |
| off, fondo estático, sin gráfico | 0,01 / 0,04 | 0,1 / 0,5 |

### 3. Fondo estático, gráfico actualizándose (dos ejecuciones)

| Vidrio | Actualización | fps mín/p10/mediana | CPU % máquina (1.ª / 2.ª) | CPU % núcleo (1.ª / 2.ª) |
|---|---|---|---|---|
| full | 1 s | 60/60/60 | 0,80 / 0,50 | 9,6 / 6,0 |
| reduced | 1 s | 60/60/60 | 0,70 / 0,68 | 8,4 / 8,1 |
| off | 1 s | 60/60/60 | 0,62 / 0,58 | 7,4 / 6,9 |
| full | 200 ms | — | 3,09 / 2,74 | 37 / 33 |
| reduced | 200 ms | — | 2,96 / 2,75 | 36 / 33 |
| off | 200 ms | — | 2,68 / 2,73 | 32 / 33 |
| full | reposo, con contador rAF | 60/60/60 | 0,36 / 0,37 | 4,4 / 4,4 |

Los JSON de la tabla 1 están en `glass-bench/results/ryzen-5-2600x/`; los de la segunda
ejecución de las tablas 2 y 3 en `…/static-and-idle/` (la primera solo se conserva en estas
tablas: el guion sobrescribió sus ficheros).

## Conclusiones

1. **El desenfoque no es el coste.** Con el fondo estático, la diferencia entre `full`, `reduced`
   y `off` queda dentro del ruido entre ejecuciones (±0,3 puntos de % máquina; en la segunda
   ejecución `full` consumió menos que `off`), tanto en reposo (0,05–0,08 % máquina) como con el
   gráfico actualizándose. En este equipo, degradar de `full` a `reduced` u `off` no ahorra
   nada medible.
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

## Equipo de referencia Intel (Core Ultra 7 155H) — 2026-09-22

**Equipo:** Intel(R) Core(TM) Ultra 7 155H (Meteor Lake, 16 núcleos/22 hilos P+E+LP-E),
gráfica integrada Intel Arc, Windows 11 Pro 26200, WebView2 Evergreen **153.0.4234.48**
(muy próxima a la 153.0.4234.32 del equipo AMD), ventana 1100×760, batería al 76 %.
Mismo banco de medida (`glass-bench/`), mismos parámetros; resultados en
`results/intel-ultra-7-155h/`. `% máquina` = tiempo de CPU / (ventana × 22).

### 1. Shell completo (fondo `.tw-ambient` animado, por defecto)

| Vidrio | Actualización | fps mín/p10/mediana | CPU % máquina | CPU % núcleo |
|---|---|---|---|---|
| full | reposo | 60/60/60 | 7,93 | 174 |
| full | 1 s | 60/60/60 | 8,72 | 192 |
| full | 200 ms | 60/60/60 | 8,57 | 189 |
| reduced | reposo | 60/60/60 | 23,29 | 512 |
| reduced | 1 s | 60/60/60 | 14,64 | 322 |
| off | reposo | 55/60/60 | 19,13 | 421 |
| off | 1 s | 60/60/60 | 23,22 | 511 |
| off | 200 ms | 60/60/60 | 17,46 | 384 |

### 2. Aislamiento del fondo animado (reposo, `measure=0`, 15 s)

| Configuración | CPU % máquina | CPU % núcleo |
|---|---|---|
| full, `.tw-ambient` animado | 13,85 | 304,6 |
| full, fondo estático | 0,35 | 7,6 |
| off, `.tw-ambient` animado | 16,35 | 359,7 |
| off, fondo estático | 0,87 | 19,2 |

### Conclusiones (Intel híbrido)

1. **Confirma la conclusión 1 del equipo AMD**: con fondo estático el coste es marginal
   (0,35–0,87 % máquina) independientemente del nivel de vidrio; degradar `full` → `off` no
   ahorra nada por sí solo. La gráfica integrada no cambia esta conclusión.
2. **El coste absoluto de `tw-ambient-drift` es 3–5× mayor que en el equipo AMD** con GPU
   discreta (13,85–23,29 % máquina aquí frente a 2,76–8,47 % en el 2600X con RTX 4060 Ti para
   configuraciones equivalentes). La gráfica integrada compone la animación con más coste de
   CPU/proceso GPU de WebView2 que una discreta. Esto **refuerza**, no contradice, la
   recomendación de pausar `tw-ambient-drift` por defecto: en el peor caso observado hasta
   ahora (iGPU) el coste ronda una cuarta parte de la máquina solo por el fondo, muy por
   encima de cualquier lectura razonable de `glass.degrade_idle_cpu_pct`.
3. **fps se mantiene en 60 en casi todos los casos** (un único mínimo de 55 en `off`/reposo,
   sin patrón claro de degradación sostenida); el umbral `glass.degrade_fps = 50` sigue sin
   poder ejercitarse con hardware real — esta iGPU tampoco lo satura con la carga del banco.
4. Con `webview2_version` casi idéntica entre equipos (153.0.4234.32 vs .48), la diferencia de
   coste se atribuye al hardware gráfico (discreta vs integrada), no a la versión del runtime.

## Umbrales `glass.*` de `spec.md`

- `glass.degrade_idle_cpu_pct = 1`: **se mantiene** como valor, pero la especificación debe
  fijar la unidad: con estos datos, «1 % de la máquina» equivale a 12 % de un núcleo en el
  equipo AMD (12 hilos) o 22 % en el equipo Intel (22 hilos) y se excede solo con el fondo
  animado en ambos equipos (0,35–0,87 % en Intel y 0,05–0,08 % en AMD con fondo estático,
  frente a 7,9–23,3 % en Intel y 2,5–8,5 % en AMD con el fondo animado); «1 % de un núcleo»
  quedaría al borde incluso sin vidrio y saltaría con cualquier ruido. Propuesta confirmada
  con dos equipos: % de la máquina, medido sobre los procesos de WebView2, **con el fondo
  ambiental pausado** (T131 no puede medir este umbral con `tw-ambient-drift` activo tal
  como está hoy, en ningún equipo; corregirlo en `design/` es requisito previo a T131).
- `glass.degrade_fps = 50` / `restore_fps = 55` y sus ventanas: **sin cambios**; los dos
  equipos de referencia (discreta y ahora integrada) se mantienen a 60 fps bajo la carga de
  este banco, así que ninguno permite ejercitar el umbral de degradación por fps.
- Los valores siguen siendo provisionales (`spec.md` los marca así hasta este spike); el
  cambio de unidad exige nueva versión de reglas cuando se decida. Con el equipo Intel medido,
  la excepción E2 de `plan.md`/CHK-L03 puede darse por resuelta en su mitad Intel para T019c;
  sigue pendiente un portátil con gráfica integrada de generación anterior si se quiere acotar
  el peor caso con más precisión.

## Pendiente

- Repetir la matriz en el Intel híbrido de referencia (GPU integrada, Windows 11 25H2) y en un
  portátil con batería. El banco y el anfitrión están listos; basta ejecutar los comandos de
  arriba y pegar los JSON.
- La aplicación real aún no arranca como ventana Tauri (sin `src/main.rs` ni CLI de Tauri;
  véase `tauri-permissions.md`), por eso se midió en un anfitrión WebView2 propio con el mismo
  runtime Evergreen.
