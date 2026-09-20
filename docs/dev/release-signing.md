# Firma del manifiesto de release (desarrollo)

Cubre la condición R2/C3 de ADR-0004: el lanzador elevado solo arranca un sidecar cuya ruta y SHA-256
figuren en un **manifiesto firmado con minisign**. Un proceso del mismo usuario puede reescribir el
binario y un manifiesto sin firma; no puede falsificar la firma.

## Piezas

| Pieza | Dónde |
|---|---|
| Módulo de verificación | `apps/desktop/src-tauri/src/release_manifest.rs` (crate `minisign-verify` 0.2.5, constitución 1.5.3) |
| Clave pública de **desarrollo** | `apps/desktop/src-tauri/keys/dev-release.pub` (ID `B88B5B4C27FA8BBF`) |
| Clave secreta de desarrollo | `%LOCALAPPDATA%\ThrottleWatch\dev-signing\dev-release.key` — fuera del repositorio, sin contraseña, solo para esta máquina de desarrollo |
| Firmante | `node scripts/sign-release-manifest.mjs --dev …` (necesita `cargo install rsign2 --locked`, probado con 0.6.6) |
| Vectores de prueba | `apps/desktop/src-tauri/tests/fixtures/release-manifest/` (manifiesto, firma válida, firma de otra clave y la clave ajena) |

## Formato del manifiesto (versión 1)

```json
{
  "manifest_version": 1,
  "version": "0.1.0-dev",
  "files": [{ "path": "SensorAgent.exe", "sha256": "<64 hex>" }]
}
```

Campos desconocidos, versiones distintas de 1, rutas no relativas o con `..`, `:` o raíz, hashes que no
sean 64 hexadecimales, duplicados (sin distinguir mayúsculas) y manifiestos de más de 64 KiB se rechazan.

## Cómo generar y verificar

```powershell
node scripts/sign-release-manifest.mjs --dev --dir apps\sensor-agent\bin\Debug\net10.0-windows `
  --version 0.1.0-dev --file SensorAgent.exe --out <carpeta de instalación> --rsign <ruta\rsign.exe>
```

Produce `release-manifest.json` y `release-manifest.json.minisig`. Comprobación cruzada con la
herramienta de referencia: `rsign verify -p apps\desktop\src-tauri\keys\dev-release.pub -x release-manifest.json.minisig release-manifest.json`.

## Cómo conectarlo en el lanzador

```rust
let key = release_manifest::trusted_public_key().ok_or("no trusted key in this build")?;
let manifest = ReleaseManifest::verify(&manifest_bytes, &signature_text, key)?; // firma primero, JSON después
manifest.check_file(&install_dir, Path::new("SensorAgent.exe"))?;               // hash del fichero
// arrancar desde ese mismo fichero (supervisor::spawn_verified), no desde una ruta resuelta de nuevo
```

- **Fallar cerrado:** `trusted_public_key()` devuelve la clave de desarrollo solo con `debug_assertions`
  o la característica `e2e`; una compilación de release no confía en ninguna clave, y por tanto rechaza
  todo manifiesto, hasta que exista la clave del actualizador y se añada aquí (T-actualizador).
- La verificación **nunca** se desactiva en desarrollo: se usa la clave de desarrollo (ADR-0004 R2).
- Dónde se instala el manifiesto y quién lo escribe (instalador por máquina, T112) lo decide la
  integración del lanzador; el directorio debe ser de solo escritura para administradores.
- TOCTOU: comprobar el hash y arrancar son dos pasos; el arranque debe partir del fichero ya
  verificado (manejador abierto o copia en un directorio protegido), no de la ruta.

## Procedimiento manual de servicio detenido (T154)

La prueba integrada usa un doble del controlador de servicios porque en el Ryzen 5 2600X
`sc.exe stop PawnIO` devuelve 1052 (el controlador no admite parada en caliente). Para repetir la
comprobación completa en un equipo donde el servicio pueda quedar detenido:

1. Reiniciar el equipo y abrir una terminal normal, no elevada; comprobar `whoami /groups` y
   `sc.exe query PawnIO` antes de iniciar ThrottleWatch.
2. Confirmar que el estado inicial es `STOPPED` y ejecutar el bootstrap de una sola UAC. El
   lanzador debe invocar `sc.exe start PawnIO`, esperar a `RUNNING` y solo entonces iniciar el
   sidecar.
3. Cerrar la sesión IPC y comprobar que el sidecar termina por EOF/latido; volver a consultar
   `sc.exe query PawnIO` y conservar la salida junto al informe de la prueba.

No se fuerza la parada ni se desinstala PawnIO en una máquina cuyo controlador no admita esa
operación; el doble de servicio cubre la secuencia determinista en pruebas.

## Qué **no** hace

- No firma releases: eso es CI protegida con la clave del actualizador (Tauri signer). Esta clave de
  desarrollo no debe usarse ni copiarse para un release.
- No sustituye Authenticode: se comprobará además cuando exista certificado (ADR-0004 C3).
