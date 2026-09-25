# Spike T028b2: decodificar la tabla PM del SMU en AMD

- Estado: **abierto**; bloqueado por una decisión de licencia y por hardware verificable
- Fecha: 2026-09-25
- Tarea: T028b2 (y su consecuencia en T173, `thermal_limit_c` ausente en AMD)
- Equipo donde se investigó: AMD Ryzen 5 2600X (Zen+, Family 17h Model 8), sesión no elevada

## Por qué existe este spike

`spec.md` FR-069 exige que, en AMD, `thermal_flag` se active cuando **el valor THM es ≥ 99 % de
su límite y PPT, TDC y EDC están por debajo del 95 % de los suyos**. `contracts/ipc-protocol.md`
añade que junto a cada bandera se emite `metadata.usage_ratio` (valor / límite).

Es decir: no basta con leer los **valores** de THM/PPT/TDC/EDC; hacen falta también sus
**límites**. Sin los dos, la regla no se puede calcular y `thermal_flag` no se puede emitir.

`LimitReasonNormalizer.IsAmdThermalFlag` ya implementa la regla, pero recibe los porcentajes ya
calculados: hoy **nadie los calcula**, porque no hay decodificador de la tabla PM. `AmdThermalTable`
solo contiene una lista de permitidos (`zen3:thermal-limits-v1`, `zen4:thermal-limits-v1`) sin
ningún dato detrás.

## Qué ofrece la librería que ya usamos

`LibreHardwareMonitorLib` 0.9.6, verificado por reflexión el 2026-09-25:

- `LibreHardwareMonitor.PawnIo.RyzenSmu` da acceso **crudo**: `GetSmuVersion()`, `GetCodeName()`,
  `ResolvePmTable(out uint, out uint)`, `UpdatePmTable()` y `ReadPmTable(int) -> Int64[]`.
  **Ningún decodificador de campos.**
- `LibreHardwareMonitor.Hardware.RyzenSMU` sí tiene un mapa de layouts
  (`_supportedPmTableVersions`), pero cubre **solo 4 versiones** y **solo valores, sin límites**:

| Versión de tabla | Sensores mapeados |
|---|---|
| `0x001E0004` | TDC, EDC, SoC (corriente y potencia), 4 temperaturas de núcleo, GFX, Fabric, Uncore, Memory |
| `0x00240903` | TDC, EDC, Fabric, Uncore, Memory, SoC (temperatura) |
| `0x00380805` | TDC, Fabric, Uncore, Memory, SoC (temperatura) |
| `0x00540004` | CPU PPT, Package (temperatura), Core/SOC/Misc/Total Power, VDDCR, TDC, EDC, relojes, L3 |

Ninguna entrada es un **límite**. Con esto no se puede calcular `usage_ratio` ni aplicar FR-069.
Conclusión: LHM no cierra el hueco.

## Fuentes externas evaluadas

Leídas directamente del código fuente (no de documentación de terceros) el 2026-09-25:

| Proyecto | Licencia | Cobertura | ¿Pares valor + límite? |
|---|---|---|---|
| [FlyGoat/RyzenAdj](https://github.com/FlyGoat/RyzenAdj) | **LGPL-3.0** | **Solo APUs**: Raven, Picasso, Dali, Renoir, Lucienne, Cezanne, Rembrandt, Phoenix, HawkPoint, KrackanPoint, StrixPoint, StrixHalo. El resto → `ADJ_ERR_FAM_UNSUPPORTED` | **Sí**: `get_tctl_temp`/`_value` (THM), `get_stapm_limit`/`_value` (PPT), `get_vrm_current`/`_value` (TDC), `get_vrmmax_current`/`_value` (EDC) |
| [hattedsquirrel/ryzen_monitor](https://github.com/hattedsquirrel/ryzen_monitor) | **AGPL-3.0** | **Sobremesa**: `0x240803`/`0x240903` (Matisse, Zen2), `0x380804`/`0x380805`/`0x380904`/`0x380905` (Vermeer, Zen3), `0x400005` (Cezanne) | **Sí**: `THM_LIMIT`, `PPT_LIMIT`, `PPT_LIMIT_FAST`, `TDC_LIMIT`, `EDC_LIMIT` (+ variantes SOC/GFX) |
| [irusanov/ZenStates-Core](https://github.com/irusanov/ZenStates-Core) | GPL-3.0, C#, usa PawnIO | Muchas versiones | **No**: su `PTDef` solo localiza fclk, uclk, mclk, VDDCR SoC, CLDO VDDP/VDDG, potencia de núcleos, VDD misc y el bloque SVI3. Ningún límite de potencia ni térmico |
| [leogx9r/ryzen_smu](https://github.com/leogx9r/ryzen_smu) | **GPL-2.0** | — | Incompatible con GPL-3.0; descartado sin evaluar más |

## Análisis de licencias

ThrottleWatch es **GPL-3.0**, fijada por la constitución.

- **RyzenAdj (LGPL-3.0): compatible sin trámite.** LGPLv3 §2 permite redistribuir la obra bajo
  GPLv3. Sirve tanto copiar el mapa de offsets a nuestro C# como enlazar la librería. No hace
  falta enmendar nada.
- **ryzen_monitor (AGPL-3.0): compatible, pero con consecuencia.** GPLv3 §13 permite combinar con
  código AGPL, pero la obra combinada arrastra las obligaciones AGPL para esa parte. Como
  ThrottleWatch es de red cero, la cláusula de red **nunca se dispararía en la práctica** — pero la
  constitución fija GPL-3.0, así que adoptarlo **exige decisión de la persona propietaria y
  probablemente enmienda de la constitución**.
- **ryzen_smu (GPL-2.0-only): incompatible** con GPL-3.0.

Nota de arquitectura: en ninguno de los casos haría falta traer el driver de esos proyectos.
LHM ya nos da la tabla cruda (`RyzenSmu.ReadPmTable`) sobre **PawnIO**, así que solo se tomaría el
**mapa de índices**. ADR-0004 queda intacto y no entra WinRing0 en el proyecto.

## Situación del equipo de desarrollo

El Ryzen 5 2600X es **Zen+ (Pinnacle Ridge, Family 17h Model 8)**:

- Su versión de tabla, medida el 2026-09-19 y registrada en `sensor-access.md`, es `0x002B0000`
  con un tamaño absurdo (3 144 159 232) — **la tabla no es legible de forma fiable en este equipo**.
- `0x002B0000` **no aparece** en ninguna de las fuentes: ni en las 4 versiones de LHM, ni en las 7
  de ryzen_monitor, ni en las familias de RyzenAdj.

Por tanto **T028b2 no se puede verificar en el equipo de desarrollo**, ni siquiera elevando.

## Hardware que sí serviría

| Familia | Ejemplos | Fuente | Licencia implicada |
|---|---|---|---|
| **APU Ryzen** (Renoir en adelante: 4000U/G, 5000U/G, 6000, 7040, 8000G) | Ryzen 7 PRO 8700GE | RyzenAdj | **LGPL-3.0, sin enmienda** |
| **Zen3 sobremesa** (Vermeer) | 5800X, 5800X3D, 5900X, 5950X | ryzen_monitor | AGPL-3.0, exige decisión |
| **Zen2 sobremesa** (Matisse) | 3600, PRO 3600, 3700X, 3800X, 3900X, 3950X | ryzen_monitor | AGPL-3.0 + ampliar el allowlist a `zen2` |
| Zen4/Zen5 sobremesa (Raphael, Granite Ridge) | 7700X, 7950X, 7950X3D, 9700X, 9800X3D, 9950X | **ninguna** | no sirve |
| EPYC y Threadripper (cualquier generación) | 9654, 7543, 7965WX… | **ninguna** | no sirve |

Regla práctica: **cuanto más nueva es la CPU, peor**. Lo verificable hoy es 2019–2021 de sobremesa,
o cualquier APU.

## Contradicción detectada en la documentación

`research.md` § 15, y por tanto **T019b**, exigen grabar el corpus etiquetado con «**AMD Zen 4**».
Pero Zen4 de sobremesa (Raphael) **no tiene mapa de campos público en ninguna fuente**, así que hoy
esa fila no es alcanzable. T019b, tal como está escrita, pide algo que no se puede cumplir.
Hay que enmendar ese texto a Zen2/Zen3 (o a un APU), que es lo que sí se puede verificar.

## Plan de verificación cuando haya máquina

Alquilada o prestada, la verificación no necesita montar la aplicación entera:

1. Instalar PawnIO y ejecutar el kit de sondeo (`docs/spikes/tools/`), elevado.
2. Registrar familia, versión y tamaño de tabla PM, y volcar la **tabla cruda completa**.
3. Contrastar los offsets candidatos de la fuente elegida contra ese volcado: comprobar que los
   valores son físicamente plausibles y que se mueven como deben bajo carga (la temperatura sube;
   PPT se acerca a su límite en carga multihilo).
4. Solo entonces implementar el decodificador, con el allowlist restringido a las versiones
   realmente comprobadas.

**Aviso:** una máquina alquilada en centro de datos sirve para el paso 3 (verificar que el
decodificador acierta), pero **no** para el corpus térmico de T019b/T045: están refrigeradas para
no hacer throttling térmico nunca y llegarán antes al límite de potencia.

## Estado y decisiones pendientes

1. **¿Se acepta la vía AGPL** (ryzen_monitor) para cubrir sobremesa Zen2/Zen3, con la enmienda de
   constitución que implica? Si la respuesta es no, la única vía limpia es **APU + RyzenAdj**, y
   entonces AMD de sobremesa se queda en nivel B indefinidamente — que es lo que `spec.md` ya
   contempla («fuera de la lista no se emiten y el equipo queda como máximo en nivel B»).
2. **Conseguir máquina** de una familia cubierta.
3. Enmendar `research.md` § 15 y T019b para que no pidan Zen4.

Hasta que 1 y 2 se resuelvan, T028b2 sigue abierta y en AMD no hay `thermal_limit_c` ni nivel A.
