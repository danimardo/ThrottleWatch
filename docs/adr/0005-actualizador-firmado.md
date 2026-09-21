# ADR-0005: Actualizador opt-in con manifiesto firmado

- **Estado:** propuesto
- **Fecha:** 2026-09-20
- **Revisores:** propietario pendiente; revisión independiente pendiente

## Contexto

ThrottleWatch no debe generar tráfico de red salvo que la persona usuaria active
el actualizador. La interfaz no puede elegir endpoints, canales, claves ni rutas
temporales. La descarga tampoco puede hacer que un artefacto alcance el estado
`verified` antes de comprobar su firma.

## Decisión propuesta

1. Usar únicamente el plugin oficial de actualizaciones de Tauri y un endpoint
   fijo de GitHub Releases:
   `https://github.com/ThrottleWatch/ThrottleWatch/releases/latest/download/update-manifest.json`
   (2026-09-21: renombrado desde `release-manifest.json`, que ya nombra el manifiesto firmado del
   sidecar, otro formato).
2. Mantener el actualizador apagado por defecto. La comprobación automática solo
   se permite al arrancar y como máximo una vez cada 24 horas; la acción manual
   es un reintento independiente.
3. Publicar un manifiesto JSON y artefactos asociados firmados con Ed25519. La
   clave privada vive exclusivamente en el secreto del workflow de publicación;
   nunca se incluye en el repositorio, el instalador ni la WebView.
4. Separar los estados `available`, `downloading`, `verified`, `installing` y
   `error`. Un parcial, una firma inválida o una interrupción descartan el
   temporal y no pueden instalarse.
5. El backend controla la descarga, la verificación y la instalación. La UI solo
   recibe estado/progreso y solicita gestos cerrados; no proporciona URL, ruta,
   ejecutable, canal ni clave.
6. La instalación exige confirmación y se bloquea durante diagnóstico,
   exportación, importación o borrado/restablecimiento.

## Alternativas descartadas

- URL configurable por la UI: permite sustituir la cadena de confianza.
- Descarga directa desde la WebView: expone red y verificación al contenido web.
- Hash fijo por máquina: no soporta versiones legítimas y no sustituye una firma.
- Actualización silenciosa: contradice el consentimiento explícito y la separación
  entre comprobar, descargar e instalar.

## Consecuencias

El endpoint y la clave pública forman parte del binario/configuración de release.
T102 necesita el secreto real del workflow; hasta entonces solo se implementan la
máquina de estados, el servidor de pruebas y los fixtures con claves efímeras.

## Implementación (2026-09-21)

- `updates.rs` (servicio puro sobre un transporte), `updates_app.rs` (plugin oficial
  `tauri-plugin-updater` 2.11.0, comandos, planificador de 24 h, operaciones que bloquean la
  instalación) y `updater.rs` (máquina de estados). El endpoint es una constante del binario; la
  clave de confianza sale de `trusted_public_key()`: la de desarrollo en compilaciones de
  depuración y `e2e`, **ninguna** en una release hasta que exista la clave de producción, de modo
  que una release no puede comprobar ni verificar actualizaciones todavía.
- `download` del plugin verifica la firma minisign (Ed25519) del artefacto antes de devolver los
  bytes: `verified` solo se alcanza después. La instalación exige `verified` y confirmación, y
  antes de lanzar el instalador se cierra el colector y la sesión en curso (el instalador de
  Windows termina el proceso sin evento de salida).
- `tauri-plugin-process` no se añade: en Windows el instalador NSIS relanza la aplicación
  (`installMode: passive`) y no hay ningún reinicio que hacer desde Rust (no verificado todavía con una instalación real: va en la
  lista manual de release).
- Pruebas: servidor HTTP local con artefacto firmado con una clave de prueba, artefacto alterado,
  clave ajena, endpoint inaccesible y actualizador apagado (0 peticiones observadas).
- Pendiente de la persona propietaria antes de publicar: generar el par de claves de producción,
  guardar la secreta como `UPDATER_SIGNING_KEY`, confirmar la pública en
  `apps/desktop/src-tauri/keys/updater-release.pub` y cablearla en `trusted_public_key()`; y
  probar `release.yml`, que no se ha ejecutado.
