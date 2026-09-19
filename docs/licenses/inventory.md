# Inventario de licencias

| Componente | Licencia | Fuente de verificación |
|---|---|---|
| ThrottleWatch | GPL-3.0 | `LICENSE` |
| Tauri | Apache-2.0 OR MIT | manifiesto Rust fijado en `apps/desktop/src-tauri/Cargo.toml` |
| Svelte / Vite / TypeScript | MIT / MIT / Apache-2.0 | manifiesto pnpm de la aplicación |
| LibreHardwareMonitorLib | MPL-2.0 | decisión registrada en `plan.md` |

El inventario completo de dependencias transitivas se generará con `cargo-about`, `cargo-deny` y
las herramientas de npm/NuGet cuando sus manifiestos estén completos.
