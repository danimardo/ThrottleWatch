# ADR-0001: Fundamentos de ThrottleWatch

**Estado:** aceptado  
**Fecha:** 2026-09-18

## Decisiones

- La aplicación se separa en frontend Svelte/Vite, backend Tauri/Rust y sidecar .NET.
- La persistencia será SQLite local y sustituible mediante puertos en Rust.
- `AnalysisChart` será SVG y recibirá series ya agregadas por el backend.
- La agregación temporal conservará extremos, media y discontinuidades explícitas.
- El motor diagnostica por niveles de cobertura; no presenta precisión de nivel A sin señales suficientes.

## Consecuencias

El trabajo inicial crea fronteras de proceso y módulos vacíos. Las implementaciones de contratos,
persistencia y diagnóstico se añadirán en los lotes posteriores, con fixtures y pruebas antes de
conectar hardware real.
