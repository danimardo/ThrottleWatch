# Spike T019/T019a: acceso a sensores y nivel de cobertura

- Estado: cerrado para T019a (2026-09-19); quedan abiertas las matrices y spikes posteriores de L03
- Fecha de inicio: 2026-09-19
- Proveedor previsto: PawnIO mediante \`LibreHardwareMonitorLib\` 0.9.6
- Ejecutable de prueba: \`apps/sensor-agent/bin/Debug/net10.0-windows/SensorAgent.exe\`

## Objetivo

Comprobar si el sidecar puede leer los registros necesarios para el nivel A y si el
proveedor permite limpiar los bits de registro sin UAC recurrente. La prueba no instala,
repara ni elimina controladores por sí sola.

## Preparación local

Desde la raíz del repositorio:

\`\`\`powershell
dotnet build .\apps\sensor-agent\SensorAgent.sln --configuration Debug --no-restore
\`\`\`

El probe se ejecuta en un modo separado del protocolo IPC normal:

\`\`\`powershell
& .\apps\sensor-agent\bin\Debug\net10.0-windows\SensorAgent.exe --probe-low-level
\`\`\`

Guardar la salida JSON en un fichero local fuera del repositorio si contiene valores de
registros de una máquina real. No registrar números de serie, nombres de usuario ni rutas.

## Qué comprueba el probe

### Intel

- PawnIO instalado y versión detectada.
- Lectura de \`0x64F\` (\`MSR_CORE_PERF_LIMIT_REASONS\`).
- Lectura de \`0x1A2\` (\`MSR_TEMPERATURE_TARGET\`) y TjMax plausible.
- Lectura de \`0x610\` (\`MSR_PKG_POWER_LIMIT\`).
- Limitación explícita de la versión actual: la API pública de LHM 0.9.6 no ofrece escritura
  de MSR, por lo que \`log_clear_supported\` aparece como \`false\` y no se declara nivel A pleno.

### AMD

- PawnIO instalado y versión detectada.
- Versión del SMU y resolución de la tabla PM.
- La lectura de la tabla y sus offsets todavía debe contrastarse con la familia concreta antes
  de añadir una versión a la lista permitida del normalizador.

## Matriz de ejecución

| Equipo | CPU/familia | Windows/build | Alimentación | PawnIO antes | Lectura sin proveedor | PawnIO instalado una vez | Tras reinicio sin UAC | Resultado A/B/C | Evidencia |
|---|---|---|---|---|---|---|---|---|---|
| Desarrollo | AMD Ryzen 5 2600X / Zen+ (Pinnacle Ridge, Family 17h Model 8) | Windows 11 Pro / 26200 | CA (sin batería detectada) | no instalado | \`missing\`, sin acceso avanzado | \`available\` (elevado): SMU \`0x002B1800\`; tabla PM no fiable | \`denied\` (usuario estándar): SMU \`0x00000000\`, \`SMU_VERSION_ZERO\` → resultado **(b)** | nivel A solo elevado; sin limpieza de bits | probes 2026-09-19 |
| Intel híbrido | Intel(R) Core(TM) Ultra 7 155H / Meteor Lake (Family 6 Model 170), 16 núcleos/22 hilos P+E+LP-E | Windows 11 Pro / 26200 | batería 76 % | ya instalado (PawnIO 2.2.0.0, servicio `Running/Manual`) | pendiente (requiere desinstalar PawnIO, no realizado) | `denied`, `MSR_READ_FAILED`, `0x64F`/`0x1A2`/`0x610` legibles pero devuelven `0x0` sin proveedor elevado | `available`: `0x1A2`=`0x0F6E0000` (TjMax 110 °C, plausible), `0x610`=`0x0042820000DD80E0` (PL1≈28 W, PL2≈64 W, ambos plausibles para este equipo), `0x64F`=`0x0` (sin razón de limitación activa en el instante de lectura); `log_clear_supported: false` (misma limitación de LHM 0.9.6 que en AMD) | nivel A de **lectura** alcanzable con sidecar elevado en Intel, con valores utilizables (mejor que AMD, que nunca superó una tabla PM no fiable); limpieza de `0x64F` sigue sin demostrarse; no probado tras reinicio | probes 2026-09-22 |
| Intel anterior | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |
| Portátil OEM | pendiente | pendiente | batería/CA | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |
| AMD Zen 4 | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | pendiente | salida JSON |

## Criterios de observación

1. La lectura se ejecuta primero sin elevar y sin proveedor instalado.
2. La instalación del proveedor, si se prueba, requiere una acción UAC separada y consciente.
3. Después de reiniciar, el sidecar se ejecuta como usuario estándar y no solicita UAC.
4. Una lectura de registro no se considera válida solo porque devuelva cero: en Intel \`0x1A2\`
   debe contener un TjMax plausible.
5. La limpieza de \`0x64F\` se considera no demostrada mientras el proveedor no permita una
   operación de escritura restringida a ese registro y no exista una prueba de lectura,
   limpieza y lectura posterior.

## Resultado y decisión

Se completará tras la ejecución real:

- \`(a)\`: nivel A sin UAC recurrente; continuar plan completo.
- \`(b)\`: nivel A requiere un servicio privilegiado; abrir revisión de amenazas antes de seguir.
- \`(c)\`: nivel A no alcanzable; continuar solo como explicador prudente B/C y retirar las
  afirmaciones de clasificación confirmada y potencial cuantificado.

La decisión debe registrarse en un ADR antes de marcar T019a como terminada.

## Ejecuciones registradas

### 2026-09-19 — equipo de desarrollo, sin PawnIO

Comando:

\`\`\`powershell
.\apps\sensor-agent\bin\Debug\net10.0-windows\SensorAgent.exe --probe-low-level
\`\`\`

Resultado relevante:

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "missing",
  "provider": "pawnio",
  "provider_version": null,
  "log_clear_supported": false,
  "details_code": "PAWNIO_NOT_INSTALLED"
}
\`\`\`

No se elevó el proceso, no se instaló ningún controlador y no se modificó ningún registro.

### 2026-09-19 — equipo de desarrollo, PawnIO 2.2.0.0 instalado (proceso elevado)

PawnIO se instaló con su instalador oficial en una acción UAC separada. Estado comprobado
antes del probe: servicio \`PawnIO\` en \`RUNNING\`, dispositivo \`ROOT\PAWNIO\0000\` con
\`Status: OK\`, \`DriverVersion 2.2.0.0\`, \`IsSigned: True\`. No se reinició el equipo.

Primera ejecución (código anterior a la corrección de este día):

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "denied",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "smu_version": null,
  "details_code": "AMD_SMU_READ_FAILED"
}
\`\`\`

Causa raíz (reproducida con un programa aparte contra LibreHardwareMonitorLib 0.9.6): la
sonda detectaba el fabricante abriendo y cerrando \`Computer { IsCpuEnabled = true }\`.
\`Computer.Close()\` llama a \`Mutexes.Close()\`, que cierra el handle de \`Global\Access_PCI\`
sin poner el campo a \`null\`; la siguiente llamada de \`RyzenSmu.GetSmuVersion()\` espera ese
mutex cerrado y LHM lanza \`TimeoutException: Timeout waiting for PCI bus mutex\`. El
\`catch\` genérico lo convertía en \`denied\`. En un proceso sin \`Computer\` la misma llamada
funciona. No tiene relación con firmas de módulos, privilegios ni reinicios pendientes.

Corrección aplicada: el fabricante se lee con CPUID (leaf 0) sin tocar LHM, y el informe
conserva etapa, tipo, mensaje y HResult de la excepción en \`error\`. Regla derivada para el
colector: no mezclar \`Computer.Open()/Close()\` con \`RyzenSmu\`/\`IntelMsr\` en el mismo
proceso.

Segunda ejecución (código corregido, mismo proceso elevado):

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "available",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [],
  "smu_version": "0x002B1800",
  "log_clear_supported": false,
  "details_code": "AMD_PM_TABLE_REQUIRES_ALLOWLIST",
  "error": null
}
\`\`\`

Observaciones:

- \`GetCodeName()\` devuelve 9 (Pinnacle Ridge). \`ResolvePmTable\` no lanza, pero devuelve
  \`version 0x002B0000\` y un tamaño absurdo (\`3144159232\`): la tabla PM de Zen+ no es
  utilizable con LHM 0.9.6 y no debe entrar en la lista permitida.
- Pendiente para cerrar la fila: ejecutar como usuario estándar tras reiniciar (criterio 3)
  y comprobar si el acceso al dispositivo requiere elevación en cada arranque.
- Decisión de producto tomada el mismo día con \`/speckit-clarify\` (\`spec.md\`, FR-087 a
  FR-090): ThrottleWatch empaqueta y ejecuta el instalador oficial de PawnIO; el nivel A se
  obtiene con un lanzador elevado exclusivo del sidecar registrado en ese paso (ADR con
  revisión de amenazas pendiente, T019a); se reutiliza un PawnIO ya instalado y nunca se
  desinstala; si el acceso falla en sesión se degrada a B/C sin interrumpir.

### 2026-09-19 — equipo de desarrollo, usuario estándar tras reiniciar (T152)

Reinicio completo del equipo; PowerShell **sin** elevar (`IsInRole(Administrator)` → `False`);
compilación limpia y ejecución de la sonda corregida:

\`\`\`json
{
  "cpu_vendor": "amd",
  "state": "denied",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [],
  "smu_version": "0x00000000",
  "log_clear_supported": false,
  "details_code": "SMU_VERSION_ZERO",
  "error": null
}
\`\`\`

Lectura: el dispositivo PawnIO se abre y el módulo carga sin excepción como usuario estándar,
pero la comunicación con el SMU devuelve cero. Comparado con la ejecución elevada del mismo día
(\`0x002B1800\`), el nivel A solo es alcanzable con el sidecar elevado. Queda confirmado el
resultado **(b)** de \`plan.md\` § Fase 0: continuar solo con un lanzador elevado exclusivo del
sidecar que supere la revisión de amenazas (ADR-0004, pendiente de aprobación por el
propietario del proyecto).

### Cierre T019a — 2026-09-19

T152 confirmó el resultado **(b)**: el SMU devuelve `0x00000000` como usuario estándar tras
reiniciar y `0x002B1800` en el proceso elevado. ADR-0004 queda aceptado con condiciones C1–C6;
por tanto T019a queda cerrada con resultado parcial y la implementación continúa por T153 → T154
→ T155, manteniendo B/C para cuentas estándar. El resto de la matriz de hardware y los demás
spikes de L03 permanecen abiertos.

### 2026-09-19 — equipo de desarrollo, servicio PawnIO en marcha (tras T020): CORRECCIÓN

**Esta entrada sustituye a una anterior del mismo día que afirmaba lo contrario y era errónea.**
La versión retirada decía que un usuario estándar leía el SMU (`0x002B1800`) con el servicio en
marcha y atribuía el `denied` de T152 a que nadie arrancaba el servicio. Esa lectura salió de una
sesión de trabajo que **estaba elevada** (nivel de integridad alto): la comprobación empleada,
`IsInRole('Administrators')`, devuelve siempre `False` en un Windows en español (el grupo se llama
«Administradores»); solo la forma `IsInRole([WindowsBuiltInRole]::Administrator)` o
`whoami /groups` es fiable.

Repetido con integridad **media** real (tarea programada `/RL LIMITED` del mismo usuario; la
etiqueta del proceso es `S-1-16-8192`, «Nivel obligatorio medio») y con `Get-Service PawnIO` →
`Running`:

| Proceso | Nivel de integridad | Servicio PawnIO | Sonda |
|---|---|---|---|
| Sesión elevada | alto | Running | `available`, SMU `0x002B1800` |
| Token restringido (`runas /trustlevel:0x20000`) | alto, sin administrador | Running | `denied`, SMU `0x00000000`, `SMU_VERSION_ZERO` |
| Tarea `/RL LIMITED` | **medio (usuario estándar)** | Running | `denied`, SMU `0x00000000`, `SMU_VERSION_ZERO` |

Lectura: con el servicio en marcha, el usuario estándar sigue obteniendo `SMU_VERSION_ZERO`; la
diferencia es el privilegio del proceso, no el estado del servicio. Queda **confirmado el
resultado (b) de T152 y ADR-0004 sigue vigente sin enmienda**: el nivel A exige el sidecar
elevado. Se descartan las opciones de «arrancar solo el servicio» que este spike y
`low-level-driver.md` llegaron a apuntar.

Regla para todas las pruebas de este spike: antes de anotar «usuario estándar», comprobar con
`whoami /groups` la etiqueta de integridad (`S-1-16-8192` = media) o lanzar la orden con
`schtasks /create … /rl LIMITED`; no confiar en `IsInRole('Administrators')`.

## Ejecución en el resto de la matriz (pendiente de hardware)

Para cada equipo de la tabla «Matriz de ejecución» que siga en `pendiente`, en ese equipo:

1. Copiar la salida de `dotnet publish apps/sensor-agent/SensorAgent.csproj -c Release -r win-x64 --self-contained -o <carpeta>`
   (o clonar el repositorio y compilar) y `docs/spikes/tools/sensor-access-matrix.ps1`.
2. Ejecutar `powershell -ExecutionPolicy Bypass -File sensor-access-matrix.ps1 -Sidecar <carpeta>\SensorAgent.exe -Phase before`
   sin PawnIO instalado y sin elevar. Genera `sensor-access-<fase>.json` con: CPU, familia,
   Windows, alimentación, sonda, 10 s de contadores PDH por procesador lógico
   (`% Processor Performance`, `Processor Frequency`, `% Processor Utility`) y catálogo LHM.
3. Instalar PawnIO con `PawnIO_setup_2.2.0.exe -install` (una UAC), repetir con `-Phase installed`.
4. Reiniciar, repetir sin elevar con `-Phase after-reboot`.
5. Traer los tres JSON (no contienen identificadores) y rellenar la fila. El contraste con
   APERF/MPERF sigue pendiente: el sidecar todavía no lee `0xE7`/`0xE8`; cuando lo haga, el
   guion añadirá esa columna.

### 2026-09-22 — equipo Intel híbrido, usuario estándar, PawnIO ya instalado

Máquina nueva: Intel(R) Core(TM) Ultra 7 155H (Meteor Lake, Family 6 Model 170), 16
núcleos/22 hilos, Windows 11 Pro 26200, con PawnIO 2.2.0.0 ya instalado y el servicio
`PawnIO` en `Running/Manual` de una sesión anterior a este spike (no se instaló nada
para esta prueba). Proceso sin elevar, nivel de integridad **medio** confirmado con
`whoami /groups` (`S-1-16-8192`) y `IsInRole(Administrator)` → `False`.

Comando: `SensorAgent.exe --probe-low-level`.

\`\`\`json
{
  "cpu_vendor": "intel",
  "state": "denied",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [
    { "name": "core_perf_limit_reasons", "address": "0x64F", "readable": true, "value_hex": "0x0000000000000000" },
    { "name": "temperature_target", "address": "0x1A2", "readable": true, "value_hex": "0x0000000000000000" },
    { "name": "package_power_limit", "address": "0x610", "readable": true, "value_hex": "0x0000000000000000" }
  ],
  "log_clear_supported": false,
  "details_code": "MSR_READ_FAILED"
}
\`\`\`

Lectura: mismo patrón que AMD (T152): sin proceso elevado, el registro se declara
`readable` pero el valor vuelve `0x0`, lo que por el criterio 4 de este spike no cuenta
como lectura válida (en Intel, `0x1A2` debería traer un TjMax plausible, p. ej. 100).
**Pendiente para cerrar la fila**: repetir en proceso elevado (UAC) para contrastar con
`0x002B1800`-tipo de valor no nulo, y repetir tras reinicio sin elevar. No se ha probado
aquí la fase «antes de instalar PawnIO» porque el proveedor ya estaba instalado de una
sesión previa; desinstalarlo para repetir esa fase no se ha hecho por ser una acción
destructiva de estado del sistema sin necesidad clara.

Ejecutado también `sensor-access-matrix.ps1 -Phase installed` (`docs/spikes/tools/sensor-access-installed.json`,
no versionado, contiene contadores locales). Hallazgo relevante para T028c
(`active_clock`/`base_clock`): `\Processor Information(*)\% Processor Performance` supera
ampliamente el 100 % en régimen turbo (hasta 371 % observado) mientras
`\Processor Information(*)\Processor Frequency` permanece clavado en tres valores fijos
por grupo de núcleo (1400 MHz en los P-core, 900 MHz en los E-core, 700 MHz en los
LP E-core), sin variar con la carga real. Esto confirma la sospecha de `research.md`: en
este equipo `Processor Frequency` es un valor nominal por grupo (no un reloj en vivo) y
el reloj activo real solo puede derivarse como `base_clock_nominal × percent_performance / 100`,
nunca leyendo `frequency_mhz` como reloj instantáneo. También confirma que
`hypervisor_present=true` en este portátil físico (VBS activo, sin hipervisor real),
corroborando la nota de `T-INT-005` sobre el bit de hipervisor de CPUID.

El contraste con APERF/MPERF sigue pendiente: el sidecar aún no lee `0xE7`/`0xE8`.

### 2026-09-22 — equipo Intel híbrido, proceso elevado (UAC), primer nivel A con valores utilizables

Mismo equipo que la entrada anterior. Elevación mediante `Start-Process -Verb RunAs`
lanzando un `.cmd` que ejecuta la sonda y redirige su salida a fichero (la política de
elevación de este equipo corporativo no mostró un cuadro de diálogo interactivo bloqueante;
el token resultante confirma `Nivel obligatorio alto` y pertenencia a
`BUILTIN\Administradores`).

\`\`\`json
{
  "cpu_vendor": "intel",
  "state": "available",
  "provider": "pawnio",
  "provider_version": "2.2.0.0",
  "registers": [
    { "name": "core_perf_limit_reasons", "address": "0x64F", "readable": true, "value_hex": "0x0000000000000000" },
    { "name": "temperature_target", "address": "0x1A2", "readable": true, "value_hex": "0x000000000F6E0000" },
    { "name": "package_power_limit", "address": "0x610", "readable": true, "value_hex": "0x0042820000DD80E0" }
  ],
  "log_clear_supported": false,
  "details_code": "MSR_READ_OK_LOG_CLEAR_UNSUPPORTED"
}
\`\`\`

Decodificación manual (criterio 4 de este spike: un registro no cuenta como válido solo
por ser distinto de cero, hay que comprobar que el valor es plausible):

- `0x1A2` bits 23:16 = `0x6E` = **110 °C** de TjMax. Plausible para este chip móvil (rango
  habitual 100–110 °C en Meteor Lake).
- `0x610` bits 14:0 = `0xE0` (224 × 1/8 W) = **PL1 ≈ 28 W**; bits 46:32 = `0x200` (512 × 1/8 W)
  = **PL2 ≈ 64 W**; ambos bits de habilitación (15 y 47) a 1. Ambas cifras son plausibles para
  un Core Ultra 7 155H en un portátil (PL1 sostenido bajo, PL2 de turbo moderado).
- `0x64F` en `0x0` es coherente: sin ninguna razón de limitación activa en el instante exacto
  de la lectura (equipo en reposo relativo), no indica fallo de lectura.

A diferencia de AMD (T152: SMU con tabla PM no fiable incluso elevado), en Intel el nivel A
de **lectura** es alcanzable con el sidecar elevado y produce cifras utilizables de inmediato.
Sigue sin demostrarse la **limpieza** de `0x64F` (misma limitación conocida de la API pública
de LHM 0.9.6 que en la entrada del 19/09) y no se ha repetido tras un reinicio del equipo.
Con esto, el resultado para Intel se acerca a **(a)** en lectura pura, aunque el ADR-0004
(resultado (b), sidecar elevado exclusivo) sigue siendo la decisión vigente porque ya
contempla el caso general (AMD no llega ni a lectura fiable sin elevar) y no se ha reabierto.

### Hallazgo 2026-09-19 — contadores PDH localizados

En este Windows en español `Get-Counter '\Processor Information(*)\% Processor Performance'`
falla con «El objeto especificado no se encontró en el equipo»: los nombres PDH están
localizados (`\Información del procesador(*)\% de rendimiento del procesador`) y
`PdhLookupPerfNameByIndex` devuelve `PDH_INVALID_ARGUMENT` para los índices del proveedor v2
«Processor Information», así que la tabla `Perflib\009` no sirve para traducirlos. El host
Rust (`active_clock`/`base_clock` derivados, contrato IPC § Muestreo) debe abrir los contadores
con `PdhAddEnglishCounterW`, nunca con la ruta localizada ni con la inglesa literal. El guion
de la matriz usa `Win32_PerfFormattedData_Counters_ProcessorInformation`, cuyos nombres son
fijos en todos los idiomas. Se anota para T-ENG (frecuencia derivada) y no se corrige aquí.
