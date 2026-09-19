# Spike T019d: permisos mínimos de Tauri

**Estado:** resuelto; lista de `capabilities/` verificada con prueba automática
**Fecha:** 2026-09-19
**Equipo:** desarrollo (AMD Ryzen 5 2600X, Windows 11 Pro 26200), Tauri 2.11.5, tauri-build 2.6.3

## Pregunta

Qué permisos necesita realmente la WebView para la barra propia, el inicio con
Windows, las notificaciones, los diálogos y el opener, y qué queda en
`capabilities/` como superficie mínima (contrato `application-commands.md`
§ Errores y seguridad: «los permisos Tauri se reducen a los comandos y API de
ventana estrictamente necesarios»).

## Método

1. Se creó `apps/desktop/src-tauri/capabilities/default.json` con una sola
   capacidad (`main-window`, ventana `main`, solo origen local).
2. `cargo check --locked` regeneró `gen/schemas/capabilities.json`; el ACL
   compilado contiene exactamente los ocho permisos de la lista.
3. `tests/acl_surface.rs` arranca la aplicación con `tauri::test::mock_builder`
   y el `generate_context!()` real (mismo ACL que producción), crea la ventana
   `main` y envía peticiones IPC como lo haría la WebView:
   - los cinco comandos de ventana de la barra propia responden `Ok`;
   - `set_size`, `set_decorations`, `set_always_on_top`, `window.create`,
     `webview.create_webview_window`, `path.resolve_directory` y `event.emit`
     devuelven `… not allowed. Permissions associated with this command: …`;
   - un comando de aplicación registrado con `generate_handler!` (`ping`) se
     invoca sin entrada en `capabilities/`.
4. Para que un ejecutable de pruebas cargue el runtime de Tauri en Windows hace
   falta el manifiesto de Common Controls v6 (`TaskDialogIndirect`); sin él el
   binario aborta con `STATUS_ENTRYPOINT_NOT_FOUND`. `build.rs` incrusta ahora el
   manifiesto mediante el enlazador MSVC (`/MANIFEST:EMBED`) en todos los
   destinos en lugar de solo en el recurso del binario. Verificado con
   `mt.exe -inputresource:throttlewatch_lib.dll;#2`.

## Resultado: `capabilities/default.json`

| Permiso | Para qué | Quién lo usa |
|---|---|---|
| `core:event:allow-listen` | `listenValidated` del puente (`collector:state`, `lifecycle:*`, `update:*`, `tray:state`, `notification:opened`) y `tauri://resize` para refrescar `maximized` | `lib/bridge` |
| `core:event:allow-unlisten` | liberar suscripciones al desmontar componentes | `lib/bridge` |
| `core:window:allow-minimize` | botón «Minimizar» de `TitleBar` | adaptador de ventana (T085) |
| `core:window:allow-toggle-maximize` | botón «Maximizar/Restaurar» | adaptador de ventana |
| `core:window:allow-internal-toggle-maximize` | doble clic en la región `data-tauri-drag-region` | Tauri, desde la región de arrastre |
| `core:window:allow-close` | botón «Cerrar»; Rust intercepta el cierre (`lifecycle:close-decision-required`) | adaptador de ventana |
| `core:window:allow-is-maximized` | estado inicial de `maximized` | adaptador de ventana |
| `core:window:allow-start-dragging` | arrastre de la ventana desde la barra | Tauri, desde la región de arrastre |

No se concede `core:default` (traería `core:app`, `core:path`, `core:image`,
`core:menu`, `core:resources`, `core:tray`, `core:webview` y `core:event:emit`).
El redimensionado por bordes lo hace el sistema en una ventana sin decoración
con `resizable: true`; no necesita `start-resize-dragging`.

## Plugins: cero permisos para la WebView

| Función | Plugin previsto (`plan.md`) | Permiso de WebView | Por qué |
|---|---|---|---|
| Inicio con Windows (T091) | `tauri-plugin-autostart` | ninguno | se llama desde `set_preference({ key: "startup.mode" })` en Rust (`app.autolaunch().enable()/disable()`); el `capabilities` nunca lista `autostart:*` |
| Notificaciones (T070) | `tauri-plugin-notification` | ninguno | `test_notification` y las alertas nacen en Rust (`app.notification().builder()`); `notification:opened` llega por evento; sin `notification:default` la WebView no puede emitir notificaciones arbitrarias |
| Diálogos de guardar/abrir (T-EXP) | `tauri-plugin-dialog` | ninguno | `export` e `import_session` abren el diálogo desde Rust con filtros fijos; la WebView nunca recibe rutas (contrato) |
| Diálogos de confirmación (cierre, borrado) | ninguno | ninguno | son componentes del sistema de diseño dentro de la WebView; los tokens de confirmación los emite Rust |
| Opener (`open_logs_folder`, `open_external_url({ id })`) | `tauri-plugin-opener` | ninguno | Rust resuelve `id` → URL de una lista cerrada (`docs`, `releases`, `licenses`) y llama `app.opener().open_url(...)`; no se concede `opener:allow-open-url` ni `opener:default`, que aceptarían URLs libres |
| Instancia única | `tauri-plugin-single-instance` | ninguno | solo backend |
| Actualizador | `tauri-plugin-updater` | ninguno | `check_for_update`, `download_update` e `install_update` son comandos propios; el plugin trabaja desde Rust; sin `updater:*` la WebView no puede fijar URL ni instalar |
| `process` (reinicio tras `reset_application`) | `tauri-plugin-process` | ninguno | `app.restart()` desde Rust; sin `process:allow-exit/restart` |
| Sidecar/procesos | `tauri-plugin-shell` | **no se incluye** | `plan.md` § Acceso de bajo nivel: la WebView no lanza procesos |

Regla derivada para T085, T091, T070 y T-INT-003: un plugin se registra en el
`Builder` de Rust para que sus API estén disponibles al backend, pero **no** se
añade ninguna entrada `<plugin>:*` a `capabilities/`. La prueba `acl_surface`
debe ampliarse con un caso negativo por plugin (`plugin:notification|notify`,
`plugin:opener|open_url`, `plugin:autostart|enable`, `plugin:process|restart`,
`plugin:dialog|save`, `plugin:updater|check`) cuando se integre cada uno.

## Verificación ejecutada

```text
cargo check --locked                                       → 0
cargo nextest run --profile ci --no-tests=pass             → 0 (65/65, incluye acl_surface 3/3)
cargo clippy --locked --all-targets -- -D warnings         → 0 avisos
cargo fmt --all -- --check                                 → 0
cargo deny check advisories licenses sources               → ok
mt.exe -inputresource:target/debug/throttlewatch_lib.dll;#2 → manifiesto Common-Controls 6.0.0.0 presente
```

`RUSTFLAGS=-A linker_messages` en todas las órdenes, como en CI.

## Hallazgos fuera del alcance (anotados, no corregidos)

- `apps/desktop/src-tauri` no tiene `src/main.rs` ni `@tauri-apps/cli` en
  `package.json`: la aplicación aún no se puede arrancar como ventana Tauri;
  bloquea T019c con la app real y T-PLAY-002.
- Sin `main.rs`, `embed-resource` enlaza el recurso de Windows como biblioteca
  nativa en vez de `rustc-link-arg-bins`; cuando exista el binario, `build.rs`
  seguirá válido (el manifiesto lo pone el enlazador, no el recurso).
