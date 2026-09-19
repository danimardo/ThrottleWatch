# ADR-0001: Arquitectura de privilegios del colector

- Estado: aceptado para L03–L04
- Fecha: 2026-09-18
- Alcance: acceso a sensores y comunicación con el colector .NET

## Contexto

ThrottleWatch debe funcionar sin elevación silenciosa, sin UAC recurrente y sin conceder a la
WebView permiso para lanzar procesos. El nivel A puede requerir acceso a registros de bajo nivel,
pero la aplicación debe seguir siendo útil en niveles B/C cuando ese acceso no esté disponible.

## Decisión

1. Rust inicia el colector .NET desde la ruta fija de `bundle.externalBin`, verifica su hash y
   canaliza `stdin`, `stdout` y `stderr` mediante `std::process::Command`.
2. La WebView no usa `tauri-plugin-shell` ni recibe permisos de proceso.
3. El colector intenta primero el modo estándar y publica una cobertura reducida si no puede leer
   registros avanzados; nunca eleva la interfaz automáticamente.
4. La instalación o reparación de un proveedor firmado de bajo nivel es una acción explícita del
   usuario y requiere UAC una sola vez. No se introduce un servicio privilegiado persistente en el
   MVP.
5. El nivel A queda condicionado a T019a: si solo funciona mediante un servicio persistente, ese
   servicio requiere una revisión de amenazas y una decisión posterior; si no es viable, el
   producto conserva las conclusiones prudentes de niveles B/C y retira cifras no demostrables.

## Consecuencias

- El flujo normal no requiere privilegios y el diagnóstico degradado es un estado soportado.
- La cobertura y el estado de acceso avanzado deben formar parte del contrato de capacidades.
- El empaquetado no puede cerrarse como nivel A hasta completar la puerta T019a en hardware real.
- La WebView queda aislada de la creación de procesos y de los secretos del sistema.

## Alternativas descartadas

- Elevar la UI en cada inicio: incumple la constitución y degrada la confianza del usuario.
- Ejecutar el colector desde la WebView: amplía innecesariamente la superficie de ataque.
- Mantener un servicio privilegiado desde el MVP: no está justificado antes del spike y la revisión
  de amenazas exigidos por el plan.
