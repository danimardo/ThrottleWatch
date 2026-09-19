# ADR-0002: Sidecar y privilegios

**Estado:** provisional  
**Fecha:** 2026-09-18

El sidecar .NET se ejecuta como proceso auxiliar y comunica por NDJSON sobre `stdin`/`stdout`,
con eventos de registro por `stderr`. El modo estándar se intenta primero. Cualquier proveedor
de acceso de bajo nivel se instalará únicamente mediante una acción explícita con UAC; la decisión
definitiva queda bloqueada por T019a.
