# ThrottleWatch — paquete SpecKit

Especificación de una utilidad de escritorio para Windows que vigila el comportamiento térmico de procesadores Intel y AMD, identifica cuándo el calor limita el rendimiento y explica la conclusión mediante datos visuales comprensibles.

> **Nombre definitivo del producto:** `ThrottleWatch`.

## Estructura

```text
.specify/
└── memory/
    └── constitution.md
historias.md
design/                         # sistema de diseño canónico, mockup y harness
specs/
└── 001-cpu-thermal-diagnostics/
    ├── spec.md
    ├── plan.md
    ├── research.md
    ├── data-model.md
    ├── ux-visual-spec.md
    ├── quickstart.md
    ├── tasks.md
    ├── gap-analysis.md         # revisión de huecos y registro de decisiones (2026-09-18)
    ├── checklists/
    └── contracts/
        ├── application-commands.md
        ├── ipc-protocol.md
        └── telemetry.schema.json
```

## Cómo consumirlo con SpecKit

1. Inicializar un repositorio con SpecKit.
2. Copiar `.specify/memory/constitution.md` a la misma ruta del repositorio.
3. Copiar `historias.md` y `specs/001-cpu-thermal-diagnostics/` a sus rutas equivalentes.
4. Pedir al agente que revise primero `constitution.md` y `spec.md`.
5. Continuar con el flujo `plan → tasks → implement → converge`.

`historias.md` reconstruye las necesidades de usuario que originan el producto. Los documentos incluyen el **qué** verificable (`spec.md`) y el **cómo** (`plan.md`, `research.md`, contratos y modelo de datos). `tasks.md` propone una secuencia implementable, pero debe revisarse después de ejecutar el análisis de coherencia de SpecKit.

## Decisiones centrales

- Aplicación local y sin nube obligatoria.
- Interfaz Tauri con frontend Svelte + TypeScript.
- Sidecar .NET que integra `LibreHardwareMonitorLib`; la GUI de LibreHardwareMonitor no se ejecuta en segundo plano.
- Acceso privilegiado aislado del proceso visual.
- Diagnóstico basado en evidencia y con nivel de confianza.
- Diferenciación explícita entre calor, límites de potencia y falta de datos.
- Porcentaje de rendimiento únicamente frente a una referencia local comparable.
- Diseño visual de alta calidad: series temporales sincronizadas, mapa térmico por núcleo, indicadores de confianza y narrativa causal.
- Sistema de diseño Svelte 5 en `design/`, con `AnalysisChart` SVG propio y agregación temporal realizada en Rust; no depende de ECharts.
- Primer inicio guiado, ajustes persistentes, interfaz completa en español e inglés y temas claro/oscuro.
- Barra de título propia, geometría de ventana restaurable y comportamiento de cierre configurable.
- Actualizaciones opcionales y firmadas desde GitHub Releases, sin red mientras estén desactivadas.

## Alcance de la primera versión

Windows 10/11 x64 y ARM64 cuando las dependencias lo permitan; CPU Intel y AMD; muestreo, historial local, diagnóstico pasivo, prueba guiada opcional, alertas, exportación y modo bandeja. GPU, overclocking, control de ventiladores, cambios de voltaje/potencia y recomendaciones automáticas de BIOS quedan fuera del MVP.
