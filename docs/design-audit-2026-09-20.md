# Auditoría de integración del sistema de diseño

Fecha: 2026-09-20.

## Método

1. `node scripts/design-sync.mjs --check` confirma que la copia de
   `design/components/`, `design/icons/`, `design/illustrations/`, `design/lib/` y los tokens
   usados por la aplicación coincide con la fuente editable de `design/`.
2. `rg -n "design/examples|\.\./examples" apps/desktop/src` confirma que la aplicación no
   importa composiciones de ejemplo.
3. `node scripts/check-catalogs.mjs` comprueba que las pantallas no introducen textos literales
   fuera de los catálogos.
4. La revisión visual de las features confirma que `Dashboard`, `Analysis`, `CpuOverview`,
   `GuidedDiagnostic`, `Sessions` y `SettingsHost` componen los adaptadores del sistema de
   diseño; no copian el CSS de los ejemplos.

## Resultado

La fuente editable sigue siendo `design/`. Las copias de runtime se regeneran con
`node scripts/design-sync.mjs`; no se editaron directamente para resolver divergencias. El
harness y el mockup son consumidores de ejemplos, no dependencias de la aplicación.

Excepciones aprobadas y acotadas:

- `SettingsHost.svelte` contiene únicamente orquestación de preferencias y puente; la apariencia
  la presta `SettingsScreen`.
- El shell coloca `TitleBar`, navegación, bandeja de avisos y el contenido de cada pantalla; esa
  composición no pertenece a un componente de pantalla del paquete de diseño.
- Las pruebas de `design/harness` incluyen fixtures locales para probar comportamiento; no se
  empaquetan ni se importan desde la aplicación.

Pendientes que no se declaran resueltos por esta auditoría: la matriz completa de axe, las
capturas visuales en ambos temas y tamaños, y cualquier puerto de monitor real. Esos criterios
pertenecen a T-A11Y-001, T-PLAY-007/008 y T087.

