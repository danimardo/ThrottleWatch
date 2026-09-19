# Spike T-PLAY-002: WebView2 por CDP

**Estado:** pendiente de ejecución en `windows-2025`  
**Fecha de preparación:** 2026-09-18

La compilación base de Tauri ya expone una página Vite y Playwright tiene los proyectos
`frontend` y `app`, pero todavía no se ha generado una compilación `e2e` de Tauri ni se ha
publicado un puerto CDP. Por tanto, no se afirma que la conducción WebView2 esté verificada.

## Procedimiento

1. Compilar con la característica `e2e` en `windows-2025`.
2. Arrancar Tauri con el puerto de depuración remoto únicamente en esa compilación.
3. Conectar Playwright mediante `connectOverCDP` y ejecutar el smoke test.
4. Registrar fps, errores de página, almacenamiento y peticiones externas.

## Alternativa si falla

Ejecutar el proyecto `frontend` contra `vite preview` para cubrir la interfaz y mantener una
suite separada de integración Tauri que arranque el ejecutable y valide IPC sin CDP.
