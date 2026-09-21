# Spike: E2E de WebView2 nativa por CDP

Fecha: 2026-09-20

## Pregunta

Determinar si Playwright puede conducir la ventana WebView2 de la compilación
`e2e` de Tauri, cuyo puerto CDP se añade en `WebviewWindowBuilder`, y dejar una
alternativa si el entorno de CI no lo permite.

## Resultado local

La prueba funciona en Windows con el binario nativo. Se verificó la siguiente
secuencia desde `apps/desktop`:

1. Arrancar Vite en `127.0.0.1:5173` con el ejecutable pnpm funcional del entorno.
2. Ejecutar `tauri dev -f e2e --no-watch` con una configuración local que deja
   vacío `beforeDevCommand` (el lanzador global `pnpm` de esta máquina no es
   ejecutable; no es una configuración que deba entrar en el repositorio).
3. Consultar `http://127.0.0.1:9222/json/version`: respondió el agente real de
   Edge/WebView2 y `webSocketDebuggerUrl`.
4. Ejecutar un script Playwright con
   `chromium.connectOverCDP('http://127.0.0.1:9222')`, localizar la primera
   página, leer su título y el DOM.

Resultado observado:

```json
{"title":"ThrottleWatch","url":"http://localhost:5173/","hasBrand":true}
```

El script terminó con código 0. La interfaz se sirvió desde la ventana Tauri
real, no desde el proyecto `frontend` de Playwright.

## Limitación pendiente

No se ejecutó un job real de GitHub Actions en `windows-2025` durante esta
sesión. T-PLAY-002 permanece abierta hasta añadir o ejecutar una comprobación
en ese runner. El proyecto Playwright existente sigue siendo un arnés Vite; no
debe presentarse como prueba de la ventana Tauri nativa.

## Alternativa si CI no puede abrir WebView2

Conservar el smoke local anterior como prueba de integración del binario y usar
el proyecto Vite únicamente para las pruebas rápidas de interfaz. En CI, el
job debe construir la variante `e2e`, arrancarla con CDP, esperar a
`/json/version`, ejecutar un smoke Playwright mediante `connectOverCDP` y
cerrar el proceso. Si el runner no dispone de WebView2 o de una sesión de
escritorio interactiva, ese job debe quedar explícitamente marcado como
infraestructura no disponible, no convertirse en una prueba falsa contra Vite.

## Arnés reutilizable (2026-09-21)

El paso 2 de arriba (`tauri dev -f e2e` con un `beforeDevCommand` vacío local)
era deliberadamente un atajo de un solo uso, no algo que debiera entrar en el
repositorio. Se sustituyó por un lanzamiento directo del binario ya
compilado, sin servidor de desarrollo:

```
pnpm exec vite build
cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml
pnpm exec playwright test --config playwright.native.config.ts
```

`custom-protocol` incrusta el `dist/` recién construido en el binario (lo
mismo que usa `tauri build`), así que no hace falta ningún servidor Vite ni
variable de entorno de máquina. `e2e-native/global-setup.ts` arranca el
`.exe`, espera a `/json/version` y devuelve una función de cierre que
Playwright llama sola al terminar; `e2e-native/fixtures.ts` sustituye el
`browser` habitual por `chromium.connectOverCDP('http://127.0.0.1:9222')`.
`e2e-native/smoke.spec.ts` confirma el título, que `__TAURI_INTERNALS__`
existe (para no confundir esta ventana con una pestaña de vista previa) y una
navegación real (Ctrl+6 → Ajustes) con IPC de verdad, no el puente falso de
`e2e/fixtures.ts`. Verificado en local (Windows) tres ejecuciones seguidas,
0 procesos `throttlewatch.exe` colgados después de cada una.

**Sigue sin hacer:** el job de `windows-2025` en CI (no verificable sin un
runner real) y decidir qué subconjunto de los especificaciones `app`
existentes (hoy arnés Vite con el puente falso) tiene sentido migrar a este
arnés — la mayoría usan parámetros de URL (`?with-history`, `?updates=on`…)
que solo entiende el puente falso, así que no se trasladan sin más. Ver T181
en `tasks.md`.
