# Spike: StrykerJS y Stryker.NET (T-MUT-003)

Fecha: 2026-09-21

Alcance pedido por la tarea: resolución de idioma (`lib/i18n`), esquemas del puente
(`lib/bridge/schemas.ts`) con StrykerJS; normalización de temperatura (`Normalization/
SensorNormalizer.cs`) con Stryker.NET. Decidir su adopción.

Ambas herramientas se instalaron temporalmente (`pnpm add -D`/`dotnet tool install -g`) solo
para este spike y se desinstalaron al terminar; no quedan en `package.json` ni como *tool*
global. `apps/desktop/stryker.spike.json` es la configuración usada, descartada tras el spike
(no se comita: no hay decisión de adopción que la justifique todavía).

## Stryker.NET — bloqueado, no evaluable todavía

`dotnet tool install -g dotnet-stryker` instala la versión 5.0.0 (la más reciente en el
momento de este spike). Al ejecutar:

```text
cd apps/sensor-agent
dotnet-stryker --mutate "Normalization/SensorNormalizer.cs" --reporter progress --reporter cleartext
```

Falla en el descubrimiento de pruebas:

```text
[ERR] TestDiscoverer: Test discovery has been aborted!
[WRN] Project '...\Tests\SensorAgent.Tests.csproj' is using Microsoft.Testing.Platform which is
      not yet supported by Stryker, see https://github.com/stryker-mutator/stryker-net/issues/3094
[INF] Number of tests found: 0 for project ...\SensorAgent.csproj. Initial test run started.
[WRN] Runner 5: Test assembly does not contain any test, skipping.
```

**Bloqueo confirmado, no una cuestión de configuración**: Stryker.NET 5.0.0 no soporta
Microsoft.Testing.Platform (el *runner* de pruebas que este proyecto adoptó explícitamente,
ver la nota histórica sobre `UseMicrosoftTestingPlatformRunner`/
`TestingPlatformDotnetTestSupport` en `Tests/SensorAgent.Tests.csproj`), y es un *issue* abierto
del propio proyecto Stryker.NET, no algo resoluble desde este repositorio. No se intentó
revertir el proyecto de pruebas a VSTest solo para el spike: habría sido una regresión
deliberada de una decisión ya tomada, y el resultado (mutantes sobre un *runner* que el
proyecto no usa) no habría sido representativo.

**Efecto secundario y su corrección**: `dotnet-stryker` reconstruyó `SensorAgent.csproj` antes
de fallar en el descubrimiento de pruebas, lo que cambió el SHA-256 de
`bin/Debug/net10.0-windows/SensorAgent.exe` e invalidó la firma del manifiesto de desarrollo
(`docs/dev/release-signing.md`) que el colector verifica al arrancar. Se restauraron el binario
y el manifiesto firmado desde una copia hecha antes del spike y se verificó la firma de nuevo
(`rsign verify`, «Signature and comment signature verified») antes de continuar: el colector
arranca exactamente igual que antes del spike.

**Decisión:** no evaluable con la versión actual de Stryker.NET. Revisar cuando el *issue*
#3094 del propio Stryker.NET soporte Microsoft.Testing.Platform, o si el proyecto necesitara
volver a VSTest por otra razón.

## StrykerJS — dos bloqueos reales, ninguno resuelto en el tiempo de este spike

`pnpm add -D --save-exact @stryker-mutator/core@10.0.0 @stryker-mutator/vitest-runner@10.0.0`
(la última versión estable de ambos en el momento del spike). Configuración
(`apps/desktop/stryker.spike.json`, descartada tras el spike):

```json
{
  "packageManager": "pnpm",
  "testRunner": "vitest",
  "vitest": { "configFile": "vitest.unit.config.ts" },
  "mutate": ["src/lib/i18n/index.ts", "src/lib/bridge/schemas.ts"]
}
```

**Primer intento** (`pnpm exec stryker run`, desde `apps/desktop`): la fase de análisis del
proyecto (`ProjectReader`) tardó **35 minutos** en encontrar «2 de 190884 fichero(s)» a mutar —
recorre el *workspace* pnpm completo, no solo `apps/desktop`, algo esperable dado
`"packageManager": "pnpm"` pero costoso en un monorepo de este tamaño. Tras instrumentar 2
ficheros con 454 mutantes, falló al arrancar el *test runner*:

```text
Error: Could not inject [class ChildProcessTestRunnerWorker]. Cause: Cannot find TestRunner
plugin "vitest". In fact, no TestRunner plugins were loaded. Did you forget to install it?
```

Causa: el descubrimiento automático de *plugins* de Stryker (que busca paquetes
`@stryker-mutator/*` instalados) no encuentra `@stryker-mutator/vitest-runner` en la estructura
de `node_modules` que produce este *workspace* pnpm (enlaces simbólicos, `node_modules`
anidados) — una categoría de problema conocida de Stryker con pnpm.

**Segundo intento**, con la causa anterior corregida declarando el *plugin* explícito
(`"plugins": ["@stryker-mutator/vitest-runner"]`): superó el análisis (mismo coste de 35 min
por el recorrido del *workspace*) y la instrumentación, y llegó a «Creating 11 test runner
process(es)» — pero no produjo ninguna salida más durante **varias horas** (de las 22:56 del
21-09-2026 a la comprobación de las 07:xx del 22-09-2026, con la sesión ya en el día
siguiente), sin errores ni progreso. Se interpretó como un bloqueo real, no una ejecución lenta
pero viva, y se terminó el proceso manualmente (`Stop-Process`/`taskkill /T /F`, confirmado sin
procesos `node` residuales).

**Decisión:** no se logró una puntuación de mutación en ninguno de los dos intentos. StrykerJS
tiene dos fricciones reales y no resueltas en este *workspace* pnpm + Svelte + Vitest: el
descubrimiento de *plugins* (solucionable declarándolos a mano, como se hizo) y un bloqueo sin
diagnóstico en la fase de ejecución de pruebas tras corregir lo anterior. No se recomienda
adoptarlo tal cual; haría falta una sesión dedicada para diagnosticar el segundo bloqueo (por
ejemplo con `--fileLogLevel trace --logLevel debug`, o probando fuera del *workspace* pnpm, en
un proyecto aislado) antes de decidir. Paquetes desinstalados (`pnpm remove`) y
`stryker.spike.json` borrado al cerrar este spike; no queda ningún rastro en `package.json` ni
`pnpm-lock.yaml`.

## Conclusión y siguiente paso (T-MUT-004)

Ninguna de las dos herramientas quedó evaluada con una puntuación de mutación real: Stryker.NET
por un bloqueo de compatibilidad ajeno a este repositorio (proyecto *upstream*), StrykerJS por
fricción sin diagnosticar en este *workspace* concreto. **No se recomienda registrar ninguna de
las dos en la constitución todavía** (T-MUT-004 depende de que aquí hubiera una decisión de
adopción, y no la hay). Próximos pasos concretos si se retoma: para Stryker.NET, esperar a que
el *issue* #3094 soporte Microsoft.Testing.Platform; para StrykerJS, diagnosticar el bloqueo de
ejecución con registro `trace` en una sesión con margen de tiempo, o probar en un proyecto
aislado fuera del *workspace* pnpm para descartar que sea la causa.
