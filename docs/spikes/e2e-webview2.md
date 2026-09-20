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
