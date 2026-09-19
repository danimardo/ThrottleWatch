# Spike T020: redistribución, instalación y retirada de PawnIO

**Estado:** ejecutado en el equipo de desarrollo; decisión legal pendiente de la persona responsable
**Fecha:** 2026-09-19
**Equipo:** AMD Ryzen 5 2600X, Windows 11 Pro 26200, sesión de trabajo elevada (corregido: véase la pregunta 3); instalador
`apps/desktop/src-tauri/resources/pawnio/PawnIO_setup_2.2.0.exe` (el que empaqueta T153)

Responde a las preguntas 2, 3 y 4 de `research.md` § 9 («Preguntas del spike»). La pregunta 1
(sensores perdidos por equipo) pertenece a T019 (`sensor-access.md`).

## Artefacto verificado

| Comprobación | Resultado |
|---|---|
| SHA-256 del instalador | `1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032` = `resources/pawnio-manifest.json` |
| Authenticode (`Get-AuthenticodeSignature`) | `Valid`; firmante `E=admin@namazso.eu, CN=namazso.eu, O=namazso, L=Debrecen, C=HU`; caduca 2027-08-05; sello de tiempo de Microsoft Public RSA Time Stamping Authority |
| Recurso de versión | `ProductName=PawnIO`, `ProductVersion=2.2.0.0`, `CompanyName=namazso` |
| Paquete de controlador instalado | `pawnio.inf` (`oem44.inf`), clase `SoftwareDevice`, versión 2.2.0.0 (2026-03-11), firmado por *Microsoft Windows Hardware Compatibility Publisher* (firma atestada; carga sin modo de prueba) |
| Modificadores del instalador (extraídos del binario) | `[-install] [-uninstall] [-unrestricted] [-debuginfo] [-silent]`; `-silent` = sin interfaz «even on error» |

El instalador se copia a sí mismo como `C:\Program Files\PawnIO\uninstall.exe` (mismo tamaño,
3 410 960 bytes) y registra `QuietUninstallString = "…\uninstall.exe" -uninstall -silent`.

## Ciclo ejecutado

Ejecutado con dos elevaciones (una por proceso) aceptadas por la persona usuaria; las sondas
intermedias son `SensorAgent.exe --probe-low-level` lanzadas desde la sesión de trabajo, que
estaba elevada. Las cifras de instalación y retirada no dependen de ello; las lecturas del SMU
solo valen para «proceso elevado» (con integridad media dan `denied`).

| Paso | Orden | Salida | Estado observado |
|---|---|---|---|
| 0 | sonda antes | — | `available`, `pawnio 2.2.0.0`, SMU `0x002B1800`, `AMD_PM_TABLE_REQUIRES_ALLOWLIST`; servicio `PawnIO` **Running/Manual** |
| 1 | `uninstall.exe -uninstall -silent` (elevado) | exit **0**, ~2 s | `C:\Program Files\PawnIO` eliminado; clave `Uninstall\PawnIO` eliminada; nodo de dispositivo y `\\.\PawnIO` desaparecidos |
| 1b | residuos tras 1 | — | entrada de servicio `PawnIO` sigue registrada (`STOPPED`, `DEMAND_START`, `WIN32_EXIT_CODE 31`) apuntando al `.sys` del DriverStore; paquete `oem44.inf` sigue en el DriverStore. Retirada completa solo tras reinicio o `pnputil /delete-driver oem44.inf /uninstall` |
| 2 | sonda tras 1 | — | `missing`, `PAWNIO_NOT_INSTALLED` |
| 3 | `PawnIO_setup_2.2.0.exe -install -silent` (elevado) | exit **0**, 1,1 s, **sin reinicio** | ficheros, clave `Uninstall` (2.2.0.0) y servicio **Running/Manual** de inmediato |
| 4 | sonda tras 3 (desde la sesión de trabajo, **que estaba elevada**; véase la corrección de la pregunta 3) | — | `available`, SMU `0x002B1800`; con integridad media real el mismo binario da `denied` / `SMU_VERSION_ZERO` |
| 5 | segundo `-install -silent` con PawnIO ya instalado (elevado) | exit **183** (`ERROR_ALREADY_EXISTS`) | sin cambios; mensaje interno «A previous installation of PawnIO was found. Please uninstall it first» |

## Respuestas

**2. ¿Es redistribuible el instalador/controlador?** Técnicamente sí: es un único ejecutable
x64 firmado, sin dependencias, con instalación y retirada silenciosas y códigos de salida
estables (0, 183). Jurídicamente, véase «Licencia» más abajo; la decisión no es de este spike.

**3. ¿Permite una instalación por máquina la lectura posterior a usuario estándar?** **No.**
Con el servicio `PawnIO` en marcha, un proceso de integridad media (usuario estándar) obtiene
`denied` / `SMU_VERSION_ZERO`, y solo un proceso elevado lee el SMU (`0x002B1800`); es el
resultado (b) de T019a, confirmado en T152 y de nuevo aquí (`sensor-access.md`, «CORRECCIÓN»).
*Corrección de una versión anterior de este documento (2026-09-19):* afirmaba que bastaba con que
el servicio estuviera en marcha y que el privilegio no importaba; se basó en sondeos lanzados
desde una sesión elevada, mal detectada como no elevada por usar `IsInRole('Administrators')`, que
en Windows en español devuelve siempre `False`. Dato que sí se mantiene: el servicio es
`DEMAND_START` y `sc sdshow PawnIO` da a `IU`/`SU` solo `CC LC SW LO CR RC` (`RP` solo para `SY`
y `BA`), es decir, un usuario estándar tampoco podría arrancarlo tras un reinicio.

**4. ¿Qué mecanismos de desinstalación y actualización exige?**

- Desinstalación: `QuietUninstallString` estándar; deja la entrada de servicio hasta el
  reinicio y el paquete en el DriverStore. ThrottleWatch no la ejecuta (ADR-0004 R6): en
  Ajustes se mostrará la ruta al desinstalador del sistema.
- Actualización: el instalador **no** actualiza sobre una instalación existente (exit 183);
  una versión inferior a la mínima exige desinstalar primero y volver a instalar, es decir,
  dos elevaciones o una tarea elevada que encadene ambas. Esto acota el estado `upgradable`
  de FR-087: se implementa como «desinstalar + instalar» dentro del mismo paso elevado.
- Detección de instalación: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`
  (`DisplayVersion`) más `C:\Program Files\PawnIO\PawnIOLib.dll`; `PawnIo.IsInstalled` de
  LibreHardwareMonitorLib coincide con este criterio (pasó a `missing` nada más desinstalar).

## Consecuencias para T153 y T154

1. `request_low_level_access` (T153) solo necesita **una** UAC y no requiere reinicio: tras
   `-install -silent` el servicio queda en marcha y el sidecar **elevado** lee nivel A en la misma
   sesión. El sidecar sin privilegios no puede (punto 3 arriba): T154 sigue siendo el lanzador
   elevado de ADR-0004, sin cambios.
2. Con un servicio `DEMAND_START` que un usuario estándar no puede arrancar, el lanzador elevado
   de ADR-0004 es también quien deja el servicio en marcha si tras un reinicio está parado
   (`sc start PawnIO` desde el proceso elevado, sin UAC porque la tarea programada ya está
   registrada). T154 debe cubrirlo con una prueba: servicio `Stopped` → sesión de nivel A →
   servicio `Running`.
3. Códigos de salida a contemplar en el comando: `0` (instalado), `183` (ya instalado),
   cualquier otro → `error` con `details_code`. Con `-silent` no hay diálogo ni en error.
4. El desinstalador de ThrottleWatch no toca PawnIO; documentar en la ayuda la retirada
   manual y que la limpieza del servicio termina en el siguiente reinicio.

## Licencia y revisión legal (hechos, sin decisión)

- `namazso/PawnIO` (código del controlador): GPL-2.0-or-later con excepción expresa para
  «independent modules that communicate with PawnIO solely through the device IO control
  interface», que pueden distribuirse siguiendo la GPL para PawnIO y la licencia propia del
  resto «provided that you include the source code of that other code when and as the GNU GPL
  requires». Excluye a los programas que hablan por la interfaz Pawn: todo módulo cargado en
  PawnIO debe ser compatible con esa licencia (recomiendan LGPL-2.1).
- `namazso/PawnIO.Setup` (releases del instalador): el README solo dice «This repository hosts
  PawnIO official releases»; el repositorio no declara licencia propia. `pawnio.eu` ofrece
  «custom licensing» por contacto.
- ThrottleWatch es GPL-3.0-only; GPL-2.0-or-later es compatible por la cláusula «or later».
- Puntos que la revisión legal debe cerrar antes de T153: (1) confirmar que empaquetar el
  instalador binario sin modificar, con su aviso de licencia y el enlace a la fuente, cumple
  la obligación de ofrecer el código fuente correspondiente de PawnIO (GPL §3) o si hace falta
  conservar una copia del código de la release; (2) si los módulos Pawn que carga
  LibreHardwareMonitorLib 0.9.6 en PawnIO están bajo una licencia compatible con la excepción
  (afecta a LHM, no a ThrottleWatch, pero es el módulo que se carga en nuestro nombre);
  (3) si procede pedir a namazso una confirmación escrita de redistribución del instalador.

## Verificación honesta

Todos los resultados anteriores proceden de órdenes ejecutadas el 2026-09-19 entre las 14:10
y las 14:12 (registros `t020-uninstall.log` y `t020-install.log` en el directorio temporal
de la sesión; contenido copiado en la tabla). No se reinició el equipo: el comportamiento tras
reinicio se toma de T152 y del descriptor del servicio, no de una nueva medición.
