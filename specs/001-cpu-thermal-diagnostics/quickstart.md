# Inicio rápido para el equipo de desarrollo

Este documento describe el estado objetivo del repositorio. Los comandos de aplicación empezarán a funcionar a medida que se completen las tareas.

## 1. Preparar SpecKit

```powershell
uv tool install specify-cli
specify init throttlewatch --integration copilot
Set-Location throttlewatch
```

Si se usa otro agente, sustituir la integración por la soportada correspondiente. Copiar después este paquete respetando `.specify/` y `specs/`.

Flujo recomendado en el chat del agente:

```text
/speckit-constitution Revisa y adopta la constitución existente.
/speckit-specify Revisa la especificación 001 sin eliminar requisitos ni decisiones confirmadas.
/speckit-plan Valida el plan técnico existente y resuelve únicamente los riesgos abiertos.
/speckit-tasks Revisa las tareas existentes, sus dependencias y cobertura.
/speckit-implement
/speckit-converge
```

## 2. Prerrequisitos de Windows

- Windows 10/11 de 64 bits actualizado.
- Rust estable con target MSVC.
- Herramientas de compilación de Visual Studio y WebView2 según los prerrequisitos de Tauri.
- Node.js LTS y gestor de paquetes fijado por el repositorio.
- SDK .NET 10 LTS.
- Git y PowerShell 7 recomendados.

No instalar controladores de sensores durante el bootstrap de desarrollo sin revisar primero el spike de privilegios y redistribución.

## 3. Variables y secretos

El desarrollo local no necesita claves de API ni conexión de red en tiempo de ejecución. Ningún secreto debe almacenarse en el repositorio. La firma de código se configura solo en CI segura.

## 4. Comandos previstos del monorepo

Una vez completado el setup:

```powershell
pnpm install --frozen-lockfile
pnpm dev
pnpm test
pnpm lint
pnpm test:e2e
cargo test --workspace
dotnet test .\apps\sensor-agent
```

`pnpm dev` debe compilar/iniciar el sidecar de desarrollo y lanzar Tauri. Debe existir también un modo replay que no requiera acceso real al hardware:

```powershell
pnpm dev:replay --trace .\packages\trace-fixtures\intel-thermal-confirmed.json
```

Antes de integrar la UI, validar también el sistema de diseño aislado:

```powershell
Set-Location .\design\harness
npm ci
npm run check
npm run build
Set-Location ..\..
```

`design/` es la fuente canónica de tokens, componentes y ejemplos. La aplicación consume una **copia sincronizada** en `apps/desktop/src/design-system/` (`pnpm design:sync`; CI comprueba la igualdad). La aplicación debe conectarla a datos reales mediante propiedades y adaptadores, sin recrear componentes casi iguales dentro de cada pantalla y sin importar nunca `design/examples/` ni `design/harness/`.

## 5. Primer vertical slice

El primer incremento aceptable hace únicamente esto:

1. Inicia sidecar y negocia protocolo.
2. Descubre CPU y cuatro métricas representativas.
3. Emite una muestra por segundo.
4. Valida y persiste en Rust.
5. Muestra estado de conexión, temperatura, carga, reloj y potencia.
6. Reproduce la misma sesión desde fixture.
7. En un perfil limpio muestra el onboarding bilingüe antes del panel; permite omitirlo sin bloquear la detección.

No incorporar aún ninguna cifra de potencial. Se añade después de implementar el nivel de cobertura, la ventana de turbo, las mesetas y el método de techo de potencia.

## 6. Datos de prueba

Todas las pruebas automatizadas deben poder ejecutarse sin sensor real mediante fixtures:

- `intel-normal`
- `intel-thermal-confirmed`
- `intel-power-limited`
- `intel-hybrid-idle-cores`
- `amd-normal`
- `amd-thermal-probable`
- `mixed-limit`
- `hot-unproven`
- `missing-effective-clock`
- `collector-disconnect`
- `suspend-resume-gap`
- `no-thermal-flag` (equipo sin bandera directa: techo `thermal_probable`)
- `derived-clock-only` (frecuencia activa derivada por contador, sin acceso avanzado)
- `battery-power-plan-change` (cambio de contexto energético a mitad de sesión)
- `virtualized-no-sensors`
- `intel-pl2-tau-drop` (fin de turbo con margen: nunca térmica)
- `laptop-dptf-chassis` (PL1 dinámico por temperatura de chasis)
- `intel-external-prochot` (PROCHOT sin THERMAL)
- `zen4-thermal-by-design` (límite térmico con frecuencia sobre la base: gravedad `boost`)
- `intel-below-base-throttle` (THERMAL con frecuencia bajo la base)
- `already-hot-start` (empieza en el límite; sin caída previa)
- `game-few-cores` (pocos núcleos activos, carga total baja)
- `ecoqos-efficiency` (frecuencia baja sin meseta ni razón: indeterminada)
- `guided-standard-intel` (sesión guiada con `throughput_ops_s`)

Los fixtures que procedan de máquinas reales deben anonimizarse y documentar consentimiento/origen. Las trazas sintéticas deben marcarse como tales.

## 7. Verificación antes de integrar

```powershell
pnpm format:check
pnpm lint
pnpm test
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
dotnet format --verify-no-changes .\apps\sensor-agent
dotnet test .\apps\sensor-agent
pnpm test:e2e
```

Además:

- validar `contracts/telemetry.schema.json`;
- reproducir corpus Intel, AMD y degradado;
- comprobar que ninguna cifra de potencial carece de método y entradas, y que ninguna traza de fin de turbo se clasifica como térmica;
- ejecutar el corpus etiquetado y sus copias degradadas (SC-003 a SC-005, SC-016 a SC-018);
- medir consumo pasivo durante una hora antes de una release candidata;
- inspeccionar tema claro/oscuro y escalas 100/125/150/200 %.
- recorrer cada pantalla en español e inglés y en anchos compacto, medio y expandido, incluidos carga, vacío, degradado y error cuando sean aplicables;
- ejecutar la comprobación y compilación de `design/harness/` y revisar que no aparezcan tokens o componentes paralelos sin excepción documentada;
- validar igualdad de claves y ausencia de literales visibles en los catálogos español/inglés;
- probar la matriz de locales `es/ca/gl/eu/ast/an/pt/en` y selección manual;
- restaurar geometría tras cambiar resolución o desconectar un monitor;
- repetir el benchmark de `AnalysisChart` con 4 pistas y 3.000 puntos por pista en hardware objetivo;
- comprobar que la agregación Rust conserva mínimos/máximos, huecos, calidad y límites de eventos al cambiar de resolución;
- confirmar que el actualizador apagado no produce tráfico y rechaza una firma inválida.

## 8. Prueba en hardware real

1. Ejecutar primero sin elevar.
2. Abrir `Cobertura` y guardar el resumen anónimo.
3. Comparar temperaturas y banderas con una herramienta de referencia.
4. No esperar el mismo instante exacto: comparar tendencia, unidad y orden de magnitud.
5. Registrar CPU, versión de BIOS, alimentación y perfil energético.
6. Si se instala acceso de bajo nivel, repetir y documentar qué capacidades aparecen.

## 9. Definición de hecho

Una historia está terminada cuando cumple escenarios de aceptación, pruebas unitarias/contrato/integración aplicables, estados degradados, ambos idiomas, temas claro/oscuro, accesibilidad, presupuesto de rendimiento y texto de diagnóstico prudente. Una pantalla bonita sin trazabilidad del resultado no está terminada.
