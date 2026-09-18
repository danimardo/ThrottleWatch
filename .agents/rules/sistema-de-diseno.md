---
paths:
  - "design/**"
  - "apps/desktop/src/**"
---

# Reglas del sistema de diseño

- `design/` es la única fuente editable de tokens, componentes, iconos e ilustraciones. La
  aplicación consume una **copia sincronizada** en `apps/desktop/src/design-system/`
  (`pnpm design:sync`, T024); nunca se edita la copia ni se importa `design/examples/` o
  `design/harness/`.
- Antes de tocar un componente, lee su sección en `design/AGENTS.md` (props exactas, reglas y
  ejemplos). No inventes props, tokens ni componentes; si falta algo, se añade en `design/` con
  ejemplo en `design/examples/` y justificación, nunca en la aplicación.
- Todo cambio en `design/` pasa `npm run check` y `npm run build` en `design/harness` y en
  `design/mockup` con cero errores y cero avisos, y actualiza `design/README.md`,
  `design/AGENTS.md` y `design-system.json` si cambian contratos.
- Textos visibles solo desde los catálogos `es`/`en` (`t('key')`); ningún literal en componentes.
- Sin `cursor: pointer` ni subrayado en botones (NFR-015); el color nunca es el único canal de
  estado; foco visible; `data-theme`, `data-motion` y `data-glass` en `<html>`.
- Versiones de `svelte`, `vite`, `typescript` y `svelte-check` en `design/harness` y
  `design/mockup` iguales a la aplicación y **exactas** (sin `^` ni `~`), según la constitución.
