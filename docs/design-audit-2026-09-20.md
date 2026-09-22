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

## Adenda 2026-09-21 (cierre de T117)

Reverificado sin cambios de fondo desde la fecha de arriba: `node scripts/design-sync.mjs --check`
(0, la copia de la aplicación sigue coincidiendo con `design/`), `rg "design/examples|\.\./examples"
apps/desktop/src` (0 coincidencias) y `node scripts/check-catalogs.mjs` (436 claves, 0 discrepancias).
Ningún componente tocado en las sesiones posteriores (T090/T161/T176/T182) introdujo estilo en
línea ni duplicado visual — todos componen `Banner`/`Button`/`Dialog` ya existentes del sistema de
diseño.

La «matriz visual completa» que quedaba pendiente en el avance del 2026-09-20 **ya existe**, solo
que en dos tareas distintas de la propia lista, no en esta: `e2e/accessibility.spec.ts` («every
primary screen passes the serious/critical axe gate») cubre las 6 pantallas principales × 2 temas
(claro/oscuro) × 3 anchos (480/840/1100) con la puerta axe serious/critical, repetido en los
proyectos `a11y-es`/`a11y-en` y `app`/`frontend` (por tanto también por idioma); `e2e/appearance-
matrix.spec.ts» cubre tema × idioma con capturas. T117 pedía auditar duplicados visuales y
documentar excepciones — no rehacer esa matriz — así que se cierra con esta adenda; la matriz en sí
sigue siendo responsabilidad de T-A11Y-001/T-E2E-09, ya marcadas en `tasks.md` con su propio alcance
y lo que a ellas les falta (el guion manual de Narrador/NVDA, ajeno a esto).

