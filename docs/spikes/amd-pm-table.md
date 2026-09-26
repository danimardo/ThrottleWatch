# Spike T028b2: decodificar la tabla PM del SMU en AMD

- Estado: **abierto**; la decisión de licencia está tomada (AGPL aceptada, 2026-09-26); bloqueado solo por hardware verificable (Zen2/Zen3 de sobremesa)
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
  ThrottleWatch es de red cero, la cláusula de red **nunca se dispararía en la práctica**.
  **Corrección 2026-09-26:** este párrafo decía que adoptarlo «probablemente» exigía enmendar la
  constitución. Releída, **no lo exige**: no fija GPL-3.0 como licencia del proyecto, solo prohíbe
  «dependencias con licencias incompatibles con GPL-3.0» (línea 399), y AGPL-3.0 es compatible por
  el §13 de GPLv3; además lo que se tomaría es un mapa de offsets, no una dependencia. Bastaba la
  decisión de la persona propietaria, que ya está tomada (véase «Estado y decisiones»).
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

## Estado y decisiones

1. **Vía AGPL (ryzen_monitor): ACEPTADA por la persona propietaria el 2026-09-26** para cubrir
   sobremesa Zen2/Zen3. No requiere enmienda de constitución (véase la corrección más arriba).
   Obligaciones que sí se derivan y hay que cumplir al implementar: la parte tomada de
   ryzen_monitor conserva su licencia AGPL-3.0 y su aviso de autoría en el fichero que la contenga,
   y debe constar en `THIRD-PARTY-NOTICES`. No soy abogado: si se distribuye fuera de este
   proyecto, conviene una revisión antes de publicar una release que la incluya.
2. **Máquina de una familia cubierta: pendiente**, y es lo que bloquea todo lo demás. El 2600X
   de desarrollo (Zen+) no vale: su tabla `0x002B0000` no está en ninguna fuente. Sirve cualquier
   Zen2 (3600, PRO 3600, 3700X, 3900X…) o Zen3 (5800X, 5900X…) de sobremesa.
3. **Enmienda de `research.md` § 15, `plan.md` y T019b: hecha el 2026-09-26.** Piden ya
   «AMD de sobremesa Zen2 o Zen3» en lugar de Zen 4.

**Siguiente paso, sin cambios:** no se implementa el decodificador hasta contrastar los offsets
contra un volcado real (paso 4 del plan de verificación de arriba). Con la máquina, el kit
`docs/spikes/tools/hardware-probe/` ya produce el volcado crudo. T028b2 sigue abierta.
