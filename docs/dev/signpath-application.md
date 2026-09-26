# Solicitud a SignPath Foundation (T113)

Borrador para `https://signpath.org/apply`. **La solicitud la envía la persona mantenedora**: va
ligada a su identidad y a la aceptación de los términos de SignPath, y no puede hacerla un agente.
Este documento solo reúne los datos y comprueba los requisitos, para que rellenarla sea cuestión
de minutos.

Fecha de la comprobación: 2026-09-26. Los términos oficiales están en `signpath.org/terms.html`;
si difieren de lo de abajo, mandan ellos.

## Requisitos comprobados

| Requisito | Estado | Cómo se comprobó |
|---|---|---|
| Repositorio público | ✅ | `gh repo view` → `PUBLIC` |
| Licencia OSI | ✅ GPL-3.0 | `gh api repos/danimardo/ThrottleWatch/license` → `GPL-3.0`, 35 150 bytes, idéntica a la canónica |
| Autenticación multifactor en GitHub | ✅ | `gh api user --jq .two_factor_authentication` → `true` |
| Política de privacidad visible | ✅ | README: «no hay telemetría remota; los datos nunca salen de tu equipo salvo que tú los exportes» |
| Pipeline de build verificable | ✅ parcial | `.github/workflows/release.yml` compila en `windows-2025` desde GitHub Actions; falta enchufar el paso de firma de SignPath (véase «Después de la aprobación») |
| Sin componentes propietarios ni malware | ⚠️ **con una salvedad** | El código propio y las dependencias son de licencia abierta (`cargo-deny`, `cargo-about`, informe npm/NuGet). **Salvedad:** el instalador incluye el instalador oficial de PawnIO 2.2.0, un binario de terceros ya firmado por su autor. Su código fuente es GPL-2.0-or-later con excepción de combinación, pero el instalador binario no tiene licencia propia publicada y la revisión legal de su redistribución sigue pendiente (`research.md` §16, T158). No sé si SignPath acepta firmar un paquete que contiene un binario de terceros: hay que declararlo y preguntarlo |
| Roles Autor / Revisor / Aprobador documentados | ⚠️ pendiente, trivial | Proyecto de una sola persona: las tres funciones las ejerce la persona mantenedora; conviene dejarlo por escrito en la solicitud |
| Código en la rama por defecto | ❌ **hoy no** | `main` solo tenía 3 commits (diseño, README, licencia); la aplicación vive en `001-cpu-thermal-diagnostics`. Se actualiza `main` antes de enviar |
| «Mantenimiento activo» y releases previas | ❓ desconocido | No hay ninguna release publicada. No sé si SignPath lo exige de forma estricta; si la rechazan por eso, se publica una preliminar y se reintenta |

## Respuestas para pegar

**Nombre del proyecto:** ThrottleWatch

**Repositorio:** https://github.com/danimardo/ThrottleWatch

**Licencia:** GPL-3.0

**Descripción (una frase):** Utilidad de escritorio para Windows que diagnostica si una CPU
Intel o AMD pierde rendimiento por calor, por potencia o por el propio equipo, y distingue esos
casos de la falta de evidencia. Sin telemetría, sin cuentas y sin red salvo un actualizador
voluntario.

**Qué se firmaría:** el instalador NSIS de Windows x64 (`ThrottleWatch_<versión>_x64-setup.exe`),
que incluye la aplicación (`throttlewatch.exe`, Tauri 2 + Rust) y su colector de sensores
(`SensorAgent.exe`, .NET 10 autocontenido). Un único artefacto por versión.

**Componente de terceros incluido (declararlo tal cual):** el instalador empaqueta el instalador
oficial de PawnIO 2.2.0 (proyecto `namazso/PawnIO.Setup`), ya firmado por su autor, que instala el
controlador que permite leer sensores. No se modifica ni se firma de nuevo; se verifica su hash y su
firma Authenticode antes de empaquetarlo, y el desinstalador nunca lo elimina. Pregunta para
SignPath: ¿es aceptable firmar el instalador propio conteniéndolo, o prefieren que se descargue
en el primer uso en vez de empaquetarlo?

**Por qué necesita firma:** el instalador es por usuario y sin elevación, pero el colector lee
sensores a través de un servicio de terceros (PawnIO) y la aplicación descarga actualizaciones
voluntarias; sin firma de código, Windows SmartScreen bloquea o desaconseja la instalación y las
personas usuarias no pueden distinguir el instalador legítimo de una copia manipulada.

**Cómo se compila:** GitHub Actions, flujo `.github/workflows/release.yml`, ejecutor
`windows-2025`, disparado manualmente por la persona mantenedora con la versión como entrada.
Compila desde el árbol del repositorio, sin pasos que descarguen código ejecutable no verificado
(el instalador de PawnIO se descarga comprobando hash y firma Authenticode).

**Datos que recoge o envía la aplicación:** ninguno. No hay telemetría. La única conexión es la
comprobación de actualizaciones, que viene **desactivada por defecto** y, mientras lo esté, no
realiza ningún tráfico de red (FR-052).

**Equipo y roles:** proyecto de una sola persona mantenedora, que ejerce a la vez de Autora,
Revisora y Aprobadora. El repositorio tiene autenticación multifactor activada.

**Contacto de la persona mantenedora:** danimardo@yahoo.es

## Después de la aprobación (lo que sí puede hacer un agente)

1. Crear en SignPath el proyecto y la política de firma; anotar su *organization ID*, *project
   slug* y *signing policy slug* (los tres se piden al configurar la acción).
2. En `release.yml`, subir el instalador como artefacto y llamar a la acción oficial
   `signpath/github-action-submit-signing-request`, con el token de la API guardado como secreto
   `SIGNPATH_API_TOKEN` (nunca en el repositorio).
3. Conservar la firma minisign del manifiesto y del actualizador, que es **independiente** de la
   firma de código: ADR-0004 no cambia, y la clave de producción sigue viviendo solo en el secreto
   `UPDATER_SIGNING_KEY`.
4. Verificar con una release preliminar que `Get-AuthenticodeSignature` da `Valid` y que
   SmartScreen no bloquea la instalación en una máquina limpia (T-QUAL-003).

Mientras no llegue esa aprobación, los instaladores que se construyan son **solo de prueba**: los
locales llevan la clave de desarrollo (`dev-signing`) y ninguno debe publicarse.
