# Informes de licencias

Los informes se generan desde las dependencias bloqueadas y no contienen secretos:

- `rust-third-party.html`: `cargo-about` con `about.toml` y `about.hbs`.
- `npm.json`: `pnpm licenses list --json` sobre el workspace.
- `nuget.json`: `nuget-license` sobre `SensorAgent.sln`, incluyendo transitivas.
- `sbom.cdx.json`: inventario CycloneDX reproducible de npm, NuGet y crates bloqueados.

La cobertura de `LibreHardwareMonitorLib` bajo MPL-2.0 y las comprobaciones previas
a publicar una release están descritas en `mpl-source-offer.md`.

CI bloquea vulnerabilidades, licencias no permitidas y fuentes no autorizadas con
`cargo-deny`. El informe de duplicados de crates queda visible mediante la comprobación
de `bans`; las duplicidades heredadas de Tauri se mantienen como aviso hasta que el
grafo de dependencias permita eliminarlas sin alterar la plataforma.
