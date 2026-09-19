# Calidad y cobertura de L00

## Suites

- Interfaz unitaria: `pnpm test:unit`.
- Componentes: `pnpm test:component`.
- Cobertura de interfaz: `pnpm test:coverage`.
- Rust: `cargo nextest run --profile ci --no-tests=pass`; `cargo llvm-cov --no-report --locked` valida la instrumentación disponible.
- Sidecar: compilar `apps/sensor-agent/Tests/SensorAgent.Tests.csproj` y ejecutar el binario xUnit v3/Microsoft Testing Platform resultante. El ejecutable publica traits `Unit` y permite formatos JUnit/XML.

## Cobertura

`coverlet.runsettings` fija el formato Cobertura para la suite .NET. Durante la excepción E1
(hasta CHK-L01 y como máximo el 2026-10-31) los umbrales se informan sin bloquear:

| Pila | General | Código crítico |
| --- | ---: | ---: |
| TypeScript/Svelte | 80 % líneas, 70 % ramas | 95 % líneas, 90 % ramas |
| Rust | 80 % líneas, 70 % regiones | 95 % líneas, 90 % regiones |
| .NET | 80 % líneas, 70 % ramas | 95 % líneas, 90 % ramas |

Los informes se conservan como artefactos de CI. Al cerrar CHK-L01, los mismos valores pasan a
ser puertas bloqueantes según la constitución XIII.
