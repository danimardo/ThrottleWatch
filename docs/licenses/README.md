# Informes de licencias

Los informes se generan desde las dependencias bloqueadas y no contienen secretos:

- `rust-third-party.html`: `cargo-about` con `about.toml` y `about.hbs`.
- `npm.json`: `pnpm licenses list --json` sobre el workspace.
- `nuget.json`: `nuget-license` sobre `SensorAgent.sln`, incluyendo transitivas.

CI bloquea vulnerabilidades, licencias no permitidas y fuentes no autorizadas con
`cargo-deny`. El informe de duplicados de crates queda visible mediante la comprobación
de `bans`; las duplicidades heredadas de Tauri se mantienen como aviso hasta que el
grafo de dependencias permita eliminarlas sin alterar la plataforma.
