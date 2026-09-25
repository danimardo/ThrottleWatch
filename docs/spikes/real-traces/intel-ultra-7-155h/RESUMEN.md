# Trazas reales — Intel Core Ultra 7 155H (Meteor Lake)

Parte de T019b (`research.md` § 15): grabaciones reales de nivel A hechas con
`docs/spikes/tools/record-real-trace.mjs`, que habla el protocolo IPC real de
`contracts/ipc-protocol.md` contra el sidecar (`--probe-low-level` no interviene aquí; es una
sesión normal de muestreo). Cada fichero es NDJSON crudo, tal cual salió de `stdout`: un
envoltorio por línea, sin editar.

- **Equipo:** Intel(R) Core(TM) Ultra 7 155H (Meteor Lake, Family 6 Model 170), 16 núcleos/22
  hilos P+E+LP-E, Windows 11 Pro 26200.
- **PawnIO:** 2.2.0.0, ya instalado, servicio `Running/Manual`.
- **Fecha:** 2026-09-25.
- **Sidecar:** build Debug local (`apps/sensor-agent/bin/Debug/net10.0-windows/SensorAgent.exe`),
  ejecutado **elevado** (proceso lanzado con `Start-Process -Verb RunAs`; sin elevar, los
  sensores `msr/*`, `cpu.package.temp` y `cpu.package.power` salen `missing`/`invalid`, igual que
  documenta `sensor-access.md`).
- **Detalle de muestreo:** `representative`, `interval_ms: 1000` (el sidecar puede ajustarlo; ver
  `started` en cada traza).

## Escenarios grabados

| Fichero | Carga (`research.md` § 15) | Duración | Cómo se generó |
|---|---|---|---|
| `multithread-render.ndjson` | Render multihilo | 60 s | 22 procesos PowerShell en bucle ocupado, uno por procesador lógico |
| `compilation-load.ndjson` | Compilación | 45 s | `dotnet build apps/sensor-agent/SensorAgent.sln --no-restore` en bucle |
| `few-cores-game.ndjson` | Juego limitado por CPU (pocos núcleos) | 45 s | 2 procesos en bucle ocupado con `ProcessorAffinity` fijada a los procesadores lógicos 0 y 1 |
| `vectorized-load.ndjson` | Carga AVX2 | 45 s | 22 procesos con un bucle de `System.Numerics.Vector4.Add`/`Multiply` |
| `hot-start-residual.ndjson` | Reposo con calor residual | 45 s (tras 20 s de precalentamiento) | Precalentamiento igual que `multithread-render` durante 20 s, luego grabación inmediata en reposo, sin dejar enfriar |

## Lo que confirman

Con proceso elevado, `cpu.package.temp`, `cpu.package.power`, `cpu.package.load`,
`cpu.package.power_limit` y las 4 banderas `msr/*` llegan con `status: "ok"` y valores plausibles
en los cinco escenarios (TjMax 110 °C ya conocido de `sensor-access.md`, `power_limit` fijo en
28 W = PL1 de este equipo). `hot-start-residual.ndjson` muestra la temperatura bajando de 74 °C a
66 °C y la carga de 14 % a 11 % en los 45 s posteriores al precalentamiento — el decaimiento
esperado de un reposo con calor residual.

## Limitaciones honestas (constitución XVI)

- **Ninguna de las cinco trazas activó `thermal_flag`, `prochot_flag`, `power_flag` ni
  `current_flag`** (los cuatro salen `false` en todas las muestras de las cinco grabaciones,
  todas las veces que se comprobó). Las cargas duraron 45–60 s con un PL1 de 28 W en un
  portátil: no fue suficiente para sostener una razón de limitación activa. Esto significa que,
  tal cual están, estas trazas **no sirven como caso "THERMAL confirmado"** del protocolo de
  `research.md` § 15 — haría falta una carga sostenida más larga (minutos) o un equipo con menos
  margen térmico para capturar un bit a `true` de verdad.
- **No hay etiquetado automático por bits, ni copias degradadas B/C, ni las otras 4 filas de
  hardware** (Intel anterior, portátil OEM, sobremesa, AMD Zen 4) — alcance decidido así para
  esta sesión; ver `tasks.md`, T019b.
- La carga «AVX2» es una aproximación con `System.Numerics.Vector4` (vectoriza en runtime, no se
  verificó a nivel de instrucción que emita AVX2 concretamente en vez de SSE). Se documenta el
  nombre del fichero como lo que es: una carga vectorizada de coma flotante, no una prueba
  instrumentada de conjunto de instrucciones.
- `record-real-trace.mjs` es una herramienta de spike sin pruebas unitarias propias, igual que
  `sensor-access-matrix.ps1` y `replay.mjs`.
