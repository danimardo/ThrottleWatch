# ThrottleWatch — Svelte design system

Svelte 5 (runes) + TypeScript implementation of the ThrottleWatch design
system, generated from its design-system artifact (tokens, brand book,
component previews and guidelines). It targets a Tauri desktop app but
has no Tauri dependency itself — the components are plain Svelte and
work the same whether the app around them is built with plain
Svelte+Vite or with SvelteKit (routing/SSR is irrelevant to a desktop
shell either way; the recommendation for a new Tauri project is plain
Svelte+Vite, kept here only as a note, not a constraint of this
package).

If you are an AI coding agent asked to wire this into an app, read
**AGENTS.md** first — it is the compact, task-oriented version of
everything below.

## What's in here

```
brand/
  README.md         Tauri wiring instructions for the app icon
  app-icon/          icon.png (1024 master), icon.ico, 32/128/256 PNGs, source.html
tokens/
  tokens.css        CSS custom properties, both themes + type styles
  tokens.ts         the same values as typed data, for non-CSS contexts
lib/
  classification.ts single source of truth for the 7 diagnostic
                     classifications -> color + icon
  responsive.svelte.ts   optional window-width tracker for the app shell
icons/
  StatusIcon.svelte fixed icon set tied to a classification
  NavIcon.svelte    the 6 sidebar/BottomBar destination glyphs (custom,
                    not generic icon-font shapes)
illustrations/
  Onboarding*.svelte  5 abstract illustrations, one per onboarding slide
components/
  Button.svelte           primary / secondary / destructive action button
  Switch.svelte           boolean toggle
  SegmentedControl.svelte 2-5 option mutually-exclusive choice
  Select.svelte           themeable dropdown listbox, for longer option lists
  Dialog.svelte           modal confirmation (native <dialog>-based)
  OptionRow.svelte        label + description + trailing control, for Ajustes
  Tooltip.svelte          keyboard + hover accessible tooltip
  ProgressBar.svelte      determinate/indeterminate linear progress track
  Banner.svelte           persistent in-page notice (info/warning/critical)
  EmptyState.svelte       centered "nothing here yet" placeholder
  OnboardingProgress.svelte  step-dot indicator for the onboarding flow
  OnboardingSlide.svelte  one slide's chrome (illustration + copy + nav)
  OnboardingFlow.svelte   the full 5-slide state machine
  SettingsScreen.svelte   the full 11-section "Ajustes" screen
  SessionCard.svelte      one session list row (status/export/delete)
  SessionsScreen.svelte   the full "Sesiones" list screen
  GuidedDiagnosticScreen.svelte  the full "Diagnóstico guiado" wizard screen
  CoreCell.svelte         one CPU core tile (temp/clock + tooltip)
  CpuTopologyMap.svelte   per-core-group grid, temp/clock toggle
  CpuAdvancedTable.svelte filterable/sortable/virtualized per-core table
  CpuScreen.svelte        the full "CPU" screen (map + table)
  AnalysisChart.svelte    hand-rolled synced multi-track SVG chart
  AnalysisScreen.svelte   the full "Análisis" screen (chart + evidence panel)
  ReportScreen.svelte     the full "Informe" narrative report screen
  CoverageMatrix.svelte   the "Cobertura" table (magnitude × available/quality/source/reason)
  ContextStrip.svelte     the always-visible strip above StatusHero (CPU, power, collector, Ver cobertura)
  ExportDialog.svelte     the one export dialog (session / report / range scopes)
  ImportResultDialog.svelte  import progress / success / error with warnings
  FirstCloseDialog.svelte the first-close question (exit vs tray-and-keep-measuring)
  CloseBlockedDialog.svelte  close requested during a guided test / export / download / install
  WhatsNewCards.svelte    post-update notices over "Ahora"
  TechnicalSummary.svelte copyable identifier-free technical summary
  LicensesScreen.svelte   third-party notices
  TitleBar.svelte
  NavigationItem.svelte
  BottomBar.svelte        compact nav: 3 direct items + a "Más" menu
  ToolbarButton.svelte
  StatusHero.svelte
  StatWidget.svelte
  StatusChip.svelte
  CausalRail.svelte
  index.ts          barrel export of all of the above
examples/
  AhoraScreen.example.svelte           the "Ahora" screen, reflowing at 3 window widths,
                                       now using NavIcon for all 6 destinations
  SettingsSection.example.svelte      a smaller slice of "Ajustes" built directly
                                       from the base primitives (segmented control,
                                       switches, a risk-zone reset) — superseded by
                                       SettingsScreen below; kept as a lighter-weight
                                       reference for the primitives it combines
  OnboardingIllustrations.example.svelte  all 5 onboarding illustrations rendered
                                       through the real components, for reference
  MorePrimitives.example.svelte       Tooltip on a CPU-core-style tile, Select in
                                       an OptionRow, ProgressBar (both states),
                                       Banner and EmptyState
  OnboardingFlow.example.svelte       the full working onboarding flow, with a
                                       demo-only control panel to switch between
                                       every detection/coverage/resumed state
  SettingsScreen.example.svelte       the full SettingsScreen component, with a
                                       demo-only control panel exercising every
                                       sensor-coverage and update-flow state
  SessionsScreen.example.svelte       the full SessionsScreen component, all 5
                                       session statuses plus the empty/error states
  GuidedDiagnosticScreen.example.svelte  the full GuidedDiagnosticScreen, a
                                       demo picker across all 12 phases + 3
                                       battery states
  CpuScreen.example.svelte            the full CpuScreen, switching between a
                                       hybrid/homogeneous/64-core topology to
                                       exercise the advanced table's virtualization
  AnalysisScreen.example.svelte       the full AnalysisScreen, synthetic tracks
                                       with a real gap, a reduced-quality run,
                                       and thermal/electrical/mixed events
  ReportScreen.example.svelte         the full ReportScreen, a demo panel toggling
                                       every notice/impact-method/causal-chain state,
                                       the provisional notice and the re-evaluated block
  ShellPieces.example.svelte          the batch-12 pieces: global banners under
                                       TitleBar, ContextStrip, CoverageMatrix, the four
                                       dialogs, WhatsNewCards, TechnicalSummary,
                                       LicensesScreen and BottomBar with «Más»
harness/
  package.json, vite.config.ts, etc.  a self-contained, npm-installable project
                                       that type-checks and builds this package
                                       against pinned exact tool versions, and
                                       serves a browsable nav over every example
                                       above — see "Reproducing the verification
                                       harness" below
```

## Brand assets (icon + illustrations)

Two more things came out of the same visual language as everything
above, not a new design pass:

- **App icon** (`brand/app-icon/`) — the thermal ring gauge (the
  product's one signature shape — the same motif as the Cover
  component in the separate ThrottleWatch visual Design System
  artifact, not a file in this Svelte package) drawn much chunkier
  than the in-app UI ring, on a rounded-square
  backdrop in the app's own dark-theme ground color. Ships as a ready
  Windows `.ico` (16 through 256px baked in) plus the individual PNGs
  Tauri's bundler expects, and a 1024px master to re-export from if you
  ever change it. `brand/README.md` has the exact `tauri.conf.json`
  wiring.
- **Onboarding illustrations** (`illustrations/`, 5 components,
  `Onboarding*.svelte`) — one per onboarding slide, built from the same
  abstract vocabulary as `Cover` (arcs, flat tinted tiles) rather than
  Corporate-Memphis-style human figures, which would read as generic
  SaaS-landing-page art next to the rest of this system's restrained,
  instrument-like language. Each is a plain SVG component with no
  props beyond optional `width`/`height`, colored entirely with this
  system's own CSS tokens so it themes for free:

  ```svelte
  <script lang="ts">
    import { OnboardingWelcome } from '$lib/design-system/components';
  </script>
  <OnboardingWelcome />
  ```

  If a future call ends up wanting literal Corporate Memphis figures
  instead, that's a real option too — just be conscious it's a
  deliberate departure from the "native app, not a web page" direction
  the rest of this system was built around, not a free upgrade.
- **Custom nav icons** (`icons/NavIcon.svelte`) — see the Components
  section below; this replaces the generic placeholder icons the first
  version of `AhoraScreen.example.svelte` used inline.

## Scope — read this before assuming a screen is "done"

This package was built in eleven batches and now covers every screen
in the spec. **Batch 1** (8 components) covers the "Ahora"
screen and the navigation chrome around it: `TitleBar`,
`NavigationItem`, `BottomBar`, `ToolbarButton`, `StatusHero`,
`StatWidget`, `StatusChip`, `CausalRail`. **Batch 2** (5 components)
adds the generic primitives almost every other screen needs: `Button`,
`Switch`, `SegmentedControl`, `Dialog`, `OptionRow`. **Batch 3** is
brand assets: the app icon, the 6 custom nav glyphs (`NavIcon`), and
the 5 onboarding illustrations. **Batch 4** adds the primitives that
exist specifically so a screen never has to fall back on raw,
OS-styled HTML elements: `Select` (a themeable dropdown), `Tooltip`
(keyboard + hover accessible, unlike a native `title`), `ProgressBar`
(now also what `StatusHero`'s own performance bar is built from),
`Banner`, and `EmptyState`. **Batch 5** is the onboarding slide flow
itself: `OnboardingProgress`, `OnboardingSlide`, and `OnboardingFlow`
— the state machine that sequences the 5 existing illustrations with
real navigation (Atrás/Siguiente/Omitir), a resumed-session note, and
slide 5's passive hardware-detection states (detecting / complete
coverage / partial coverage / advanced access unavailable), all driven
by props — `OnboardingFlow` runs no real detection itself. **Batch 6**
is the full "Ajustes" screen: `SettingsScreen`, one exported component
covering all 11 sections the spec describes (General, Idioma,
Apariencia, Monitorización, Bandeja y notificaciones, Datos y
privacidad, Sensores y cobertura, Diagnóstico, Actualizaciones, Acerca
de y ayuda, Zona de riesgo), built entirely from the batch 1–4
primitives above with no new ones added and no changes to the
primitives themselves.

**Batch 7** is the "Sesiones" screen: `SessionCard` (one session row —
active/completed/cancelled/incomplete/imported, with export/delete
actions and a live-pulsing dot for the active state) and
`SessionsScreen` (loading/error/empty/loaded list, owning one shared
delete-confirmation `Dialog` rather than one per card). **Batch 8** is
the "Diagnóstico guiado" wizard: `GuidedDiagnosticScreen`, a single
component driven entirely by a `phase` prop (12 values, preflight
through result/cancelled/error/safety-stop/sensor-lost), rendering its
own fixed 5-step stepper and a battery-state banner. **Batch 9** is the
"CPU" screen: `CoreCell` (one core tile with a keyboard+hover
`Tooltip`), `CpuTopologyMap` (a per-core-group grid that renders a
homogeneous CPU as one ungrouped group and a hybrid one as 2–3 labeled
groups, with no special-cased branching between the two shapes),
`CpuAdvancedTable` (filterable, sortable, and hand-rolled-virtualized —
no virtualization library), and `CpuScreen` (composes the two).
**Batch 10** is the "Análisis" screen: `AnalysisChart` (a hand-rolled
SVG chart syncing temperature/clock/load/power on one time axis, one
shared hover cursor, gap-aware line segments that never interpolate
across a missing sample, dashed reduced-quality segments, event bands
including a genuine two-color diagonal pattern for a `'mixed'` thermal
+ electrical event, and a drag-to-select zoom/range brush) and
`AnalysisScreen` (wraps it with a `noHistory`/`loading`/`ready` status,
an optional stale/partial-data banner, and an evidence panel that
prioritizes a selected event's evidence, then a selected range's, then
an idle hint). **Batch 11** is the "Informe" screen: `ReportScreen`, a
narrative report — resultado en una frase, qué se observó, impacto
estimado, evidencias, causas alternativas, qué no puede concluirse,
recomendaciones, método del impacto, comparación antes/después — where
every section with no content is omitted entirely rather than rendered
as an empty stub, and the optional causal-chain block reuses
`CausalRail` under that component's own "only with real evidence"
rule.

**Batch 12** (2026-09-18) is the outcome of the spec review recorded in
`specs/001-cpu-thermal-diagnostics/gap-analysis.md` §10 (tasks
T118–T129): nine new components — `CoverageMatrix`, `ContextStrip`,
`ExportDialog`, `ImportResultDialog`, `FirstCloseDialog`,
`CloseBlockedDialog`, `WhatsNewCards`, `TechnicalSummary`,
`LicensesScreen` — plus a `menu` ("Más") prop on `BottomBar`, a
`size="wide"` on `Dialog`, the shared `AdvancedAccessState` enum in
`lib/access.ts`, and contract changes to `SettingsScreen` (tri-state
close action, no crash-report toggle, no update channel, `verified`
update state with separate Download/Install, per-section folded
"Avanzado" blocks, motion, on-battery, storage usage, export, repeat
intro, embedded `coverage`/`technicalSummary` snippets),
`OnboardingFlow`/`SettingsScreen.sensors` (`advancedAccess` enum instead
of a boolean), `GuidedDiagnosticScreen` (6-step stepper with an optional
`rest` phase, structured `whatWillHappen` intro, remaining time,
headroom, use-as-reference) and `SessionCard`/`SessionsScreen`/
`ReportScreen` (reference marking, Importar…, provisional notice,
re-evaluated block). The "global banners under `TitleBar`" pattern
(guided test in progress, collector degraded) is a shell composition of
`Banner`, shown in `ShellPieces.example.svelte` and `mockup/`.

Every screen the spec describes now has a dedicated component. If a
future task needs something none of the components below cover,
that is new scope, not a gap in this inventory — say so rather than
approximating it with an unrelated component.

## Installing it into a Tauri + Svelte project

There is no build step and no npm package here on purpose (by request,
this ships as a plain folder rather than a publishable package). Copy
the whole `throttlewatch-design-system` folder into your project's
`src/lib/` (SvelteKit) or `src/` (plain Svelte+Vite), e.g.:

```
src/lib/design-system/   <- this folder's contents go here
```

Then, once near the root of the app (root `+layout.svelte`, or
`App.svelte` / `main.ts` for plain Svelte):

```ts
import '$lib/design-system/tokens/tokens.css';
// or, plain Svelte+Vite: import './lib/design-system/tokens/tokens.css';
```

Set the theme attribute wherever you toggle dark/light (defaults to
dark if you never set it):

```ts
document.documentElement.dataset.theme = 'dark'; // or 'light'
```

Import components from the barrel:

```svelte
<script lang="ts">
  import { StatusHero, StatusChip, StatWidget } from '$lib/design-system/components';
</script>
```

No dependencies beyond Svelte itself — no icon library, no CSS
framework, no font download (see "Typography" below).

## Content fundamentals (carry these into any screen you build)

- ThrottleWatch observes and diagnoses; it **never** implies it changed
  voltages, clocks, BIOS settings or fans. No UI copy should imply an
  action on the hardware.
- A conclusion always distinguishes three levels: **observation**
  (what a sensor measured), **inference** (a temporal correlation) and
  **confirmation** (a persistent direct signal). Never blend these into
  one ambiguous sentence.
- A missing value is stated explicitly ("No disponible" / "Not
  available") — never replaced with `0`, never a hidden row.
- There is no "available performance %". The only figures are the
  cooling potential from the power-headroom method (level-A equipment)
  and the measured guided-test result.
  `StatusHero`'s `performance` prop enforces this: omit it and the
  component renders the "no cuantificable" fallback text instead of letting
  you fabricate a number.
- Everything ships bilingual ES/EN from day one. **This package
  contains no user-facing copy** — every string is a prop the
  consuming app supplies from its own translation catalog. Do not
  hardcode Spanish or English text inside these components.
- Tone: calm, precise, never alarmist. A severe thermal state is
  communicated with color and hierarchy, not exclamation marks or
  dramatic adjectives.

## Visual foundations

**Why this language.** Apple/HIG vocabulary (glass, system typography,
system colors, hierarchy by space rather than decoration) because
ThrottleWatch is fundamentally a fast-read instrument: the user needs
to know in seconds whether something is wrong and why.

**This is a Windows app.** `TitleBar` does NOT replicate macOS's
red/amber/green traffic-light window controls — that is a highly
recognizable piece of Apple's own brand identity, and this app ships
its own monochrome minimize/maximize/close icons of equal visual
weight instead, per the functional spec.

**Native, not web.** No action ever grows a hover underline, and no
button, tab, chip or link-styled row ever shows a hand cursor —
`cursor: default` throughout, matching how real Windows controls
behave, with the one documented exception of `AnalysisChart`'s
plotting surface (crosshair/pointer while scrubbing or hovering a
clickable event). In-app actions are `<button>`, not `<a href>`. See
`tokens.css`'s "Native-app affordances" comment and `AGENTS.md` rule 27
for the full rule and its rationale.

**Typography is the system stack, not San Francisco.**
`-apple-system` only activates on an Apple device; on Windows this
exact `font-family` list resolves to **Segoe UI**, the real native
typeface of the target platform. This is deliberate: the design
inherits Apple's typographic rhythm (weights, tracking, scale) without
pretending to a font a Windows user would never actually see. Do not
add a web font.

**Color.** Status colors are Apple's real system colors (red, orange,
green, purple, gray) because they map naturally onto the functional
spec's own semantics (thermal=red, warning=orange, power=purple,
normal=green, unknown=gray) and read instantly. In **light** theme they
are deliberately deepened from Apple's vivid tone — Apple's vivid red/
orange/green/purple fail 4.5:1 on white (some don't even clear 3:1), so
every `status-*` light value is a deeper variant chosen to pass AA as
text, not just as a mark. In **dark** theme the official vivid Apple
values are used as-is — they're already calibrated for that contrast
on a near-black background.

`accent-blue` is the system color — navigation, focus, toolbar
actions, neutral informative figures (available performance is a
fact, not an alert) — and must **never** represent a thermal or power
alert state. Keeping that boundary is what lets blue dominate the
interface without diluting the semantic colors.

Tinted backgrounds (chips, icon tiles) are not their own tokens: they
are the relevant `status-*` or `accent-blue` at 10–16% opacity over
`surface`, generated with `color-mix()` at the point of use (see any
component's `<style>` block) — never documented as standalone colors,
so they can't get used loose without their paired text/icon.

**Glass material (batch 13, 2026-09-18).** Surfaces are glass, in the
spirit of Apple's Liquid Glass but tuned for a Windows instrument that
must stay legible: high-alpha translucent fills (80 % dark / 76 % light
for content cards, 90 % / 88 % for floating chrome), an 18px backdrop
blur with 1.35 saturation, a 1px luminous edge, a faint diagonal
specular sheen and one soft elevation shadow. Three strengths —
`--glass-bg` (cards), `--glass-bg-strong` (TitleBar, BottomBar and its
menu, Dialog, Tooltip, Select popup, side panels), `--glass-bg-subtle`
(secondary Button, SegmentedControl, CoreCell) — all derived from
`--glass-rgb` + `--glass-alpha*` per theme. The app shell mounts
`.tw-ambient` (three slowly drifting tinted blobs) as the thing the
glass refracts; screens themselves are transparent. Quality levels via
`document.documentElement.dataset.glass = 'full' | 'reduced' | 'off'`
(helpers `applyGlassLevel`/`applyMotionLevel` in `tokens.ts`):
`reduced` drops the blur, `off` is the old flat look with hairlines;
no-`backdrop-filter` browsers fall back to 94–97 % alpha automatically.
Status colors, the thermal ring, event bands and reduced-quality
patterns are never glass — the material is the support, not the message.

**Elevation.** A glass surface carries its own `--glass-shadow` (one
soft shadow + the inset edge light). There is still no generic
`--shadow-*` family and flat elements keep the 1px `--hairline` rule —
don't add ad-hoc `box-shadow`s.

**Iconography.** Line icons, `stroke-width` 1.8–2.2, round
`stroke-linecap`/`stroke-linejoin`, no fill except very small state
dots (6–7px). A status icon always lives inside a rounded tile
(`radius-sm` or `radius-md`) tinted to its semantic color — never a
flat-colored icon floating directly on `surface`.

**Typography and numbers.** Numeric values (temperature, clock, power,
percentage) always use `font-variant-numeric: tabular-nums` (baked
into the `.value-*` classes in tokens.css) so digits don't shift width
on live updates. The type scale is deliberately short — eight styles
total: this is a fast-read utility, not a text editor.

**Motion.** Movement lives in transitions and state changes; rest is
calm. Tokens: `--motion-fast` 160ms, `--motion-base` 240ms,
`--motion-slow` 420ms, `--motion-spring` (slight overshoot),
`--motion-stagger` 45ms. Utilities `.tw-enter` (rise + fade, staggered
by `--tw-i`) and `.tw-pop` (spring scale). What animates: screen entry
(shell), staggered entry of StatWidget/coverage rows/WhatsNew/sessions/
report blocks/settings sections, the thermal ring arc + colour + centre
value swap + classification tag pop, a value-change bump on StatWidget,
ProgressBar spring fill with one specular sweep, BottomBar menu "drop",
Dialog pop with blurred backdrop, Tooltip/Select pop, liquid press
(97 % scale + radial ripple from the pointer on Button), event bands
unfolding in AnalysisChart, CausalRail connectors drawing, a
reconnection ring on ContextStrip, onboarding slide pop/rise, hover
lift on cards and cores. The one continuous motion at rest is the
thermal ring's halo breathing (6s, low amplitude). The value text
always reflects the real figure, never an interpolated number.
`tokens.css` collapses everything to instant under
`prefers-reduced-motion: reduce` and under
`document.documentElement.dataset.motion = 'reduced'`; `'full'`
explicitly overrides the OS preference.

## Compacting and stretching

ThrottleWatch must live in a narrow utility window as well as a wide
working window, without losing capability — only reordering it. Three
width tiers (see `lib/responsive.svelte.ts` / `tokens.ts BREAKPOINTS`):

| Width | Navigation | Hero | Metrics | Causal rail |
|---|---|---|---|---|
| **< 700px** (compact) | `BottomBar`, "Ahora" always reachable | column | 1-col grid | hidden (progressive detail) |
| **700–979px** (medium) | icon-only rail (`NavigationItem` with no label) | row | 2-col grid | hidden |
| **≥ 980px** (expanded) | labeled sidebar with section header | row + separate perf block | 4-col grid | visible |

Two different mechanisms produce this, and it matters which is which:

- **`StatusHero`'s row→column switch is internal**, driven by a CSS
  container query on the component's own rendered width (900px), not
  the window's. You never pass it a layout prop.
- **Everything else in the table above is the app shell's decision.**
  `NavigationItem`'s `density`, whether you render `BottomBar` or the
  sidebar, whether you mount `CausalRail` at all — these come from
  tracking the *window's* width, which `lib/responsive.svelte.ts`'s
  `createWidthTracker()` does for you (optional convenience, not
  required).

The `StatWidget` grid uses
`grid-template-columns: repeat(auto-fit, minmax(150px, 1fr))`, so 1–4
columns emerge from available width with no dedicated breakpoint for
that component specifically.

## Components

**Base primitives** (new in this batch):

- **Button** — primary (solid accent-blue) / secondary (tinted +
  hairline) / destructive (solid status-thermal) action button. Not a
  marketing-pill CTA — a native rounded-rect shape. `destructive` is
  reserved for the confirm button *inside* a `Dialog`, never for the
  row that opens it (see the component's own doc comment for why —
  it's about keeping `status-thermal` meaning one thing everywhere).
- **Switch** — a boolean toggle for the many on/off settings.
- **SegmentedControl** — a 2–5 option exclusive choice (language,
  appearance, monitoring profile, retention period), styled as
  connected segments rather than a dropdown.
- **Dialog** — the one modal primitive (native `<dialog>`-based):
  exit-to-tray choice, destructive confirmations, update-install
  confirmation.
- **OptionRow** — label + description (or disabled-reason) + trailing
  control, the row shape the whole Ajustes screen is made of.
- **NavIcon** (`icons/NavIcon.svelte`) — the 6 destination glyphs
  (`now` / `analysis` / `cpu` / `sessions` / `guided` / `settings`),
  each drawn to be specific to what its screen actually is (the ring
  gauge for "Ahora", synced sparkline tracks for "Análisis", a die with
  differently-sized P/E blocks for "CPU"...) rather than generic
  icon-font shapes. Pass it inside a snippet: `{#snippet icon()}<NavIcon kind="now" />{/snippet}`.

**More primitives** (batch 4 — added to avoid raw HTML elements
leaking their OS-styled default chrome into what should read as a
native app):

- **Select** — a themeable dropdown listbox for option lists too long
  to fit as `SegmentedControl` segments. Not built on a native
  `<select>`, whose popup is painted by the OS/browser outside any CSS
  this system controls. Fully keyboard-operable (arrows, Enter/Space,
  Escape).
- **Tooltip** — keyboard- and hover-accessible, and doesn't disappear
  the instant the pointer leaves the trigger — a native `title`
  attribute fails both of those, which the spec explicitly requires
  (the CPU topology map's per-core readout, Análisis's chart
  tooltips). You supply the trigger element yourself via its
  `children` snippet.
- **ProgressBar** — the 4px hairline-bounded linear track, determinate
  or `indeterminate`. `StatusHero`'s own performance bar is now built
  on this instead of its own inline markup.
- **Banner** — a persistent in-page notice (connection failure,
  low-power mode active, update available) — not a toast, not a
  `Dialog`. Draws its own 3 small icon shapes rather than reusing
  `StatusIcon`, whose glyphs are reserved for the 7 diagnostic
  classifications. The dismiss (×) button's accessible name is a
  `dismissLabel` prop, required whenever `onDismiss` is set — no copy
  lives in this package (see AGENTS.md §0.5 rule 2).
- **EmptyState** — a centered "nothing here yet" placeholder (Análisis
  before enough history, a filtered list with no matches), no card or
  border by default.

**Onboarding flow** (batch 5):

- **OnboardingFlow** — the state machine: sequences the 5 existing
  illustrations in their fixed order, wires Atrás/Siguiente/Omitir,
  and renders slide 5's detection block. Presentational only — pass
  `initialStep` for a resumed session, and drive slide 5's `steps[4]`
  object with your own real detection status (`'detecting' |
  'complete' | 'partial'`), coverage text, and
  `advancedAccessAvailable`. It never performs real hardware detection
  itself.
- **OnboardingSlide** — one slide's chrome (illustration, title, body,
  optional `extra` block, progress dots, nav buttons). Use directly
  only if you're assembling a flow that doesn't fit `OnboardingFlow`'s
  fixed 5-slide shape.
- **OnboardingProgress** — the step-dot row (done / current / upcoming)
  `OnboardingSlide` uses internally; exported in case you need it
  elsewhere.

**Ajustes screen** (batch 6):

- **SettingsScreen** — the full 11-section settings screen, one
  exported component. Every current value, option list, disabled
  reason, and status (sensor coverage, update progress, delete/reset
  progress) is a prop — see `AGENTS.md` section 2 for the full prop
  reference of its 11 section objects
  (`general`/`language`/`appearance`/`monitoring`/`tray`/`privacy`/`sensors`/`diagnostics`/`updates`/`about`/`riskZone`).
  Reuses `OptionRow`/`Switch`/`SegmentedControl`/`Select`/`Button`/
  `Dialog`/`ProgressBar`/`Banner` as-is; its only local state is
  whether its two destructive-action confirmation dialogs ("eliminar
  todos mis datos", "restablecer ThrottleWatch") are open. Reflows at
  its own container width via a CSS `@container` query (not
  `window.innerWidth`), so it stays correct however it's hosted.

**Sesiones screen** (batch 7):

- **SessionCard** — one session row: a live-pulsing dot for `active`, a
  `StatusChip` for `completed`/`imported` with a classification, a
  muted tag otherwise, plus export/delete icon buttons. Delete fires
  `onRequestDelete` immediately with no confirmation of its own — the
  screen owns the one shared confirm dialog.
- **SessionsScreen** — the loading/error/empty/loaded list built from
  `SessionCard` + `EmptyState` + `Banner` + one shared delete
  confirmation `Dialog`. Presentational: `status` and the `sessions`
  array are entirely host-computed.

**Diagnóstico guiado screen** (batch 8):

- **GuidedDiagnosticScreen** — a single component driven by a `phase`
  prop (12 values covering preflight through result and every
  interruption: cancelled, safety-stop, sensor-lost, error). Renders
  its own fixed 5-step stepper and a battery-state `Banner`.
  Presentational, same contract as `OnboardingFlow`/`SettingsScreen` —
  it never runs the diagnostic itself. The stepper's 5 captions, its
  `aria-label`, the preflight-failed banner title, the preflight
  progress label, and the two live-reading `StatWidget` labels
  (Temperatura/Límite) are all props too, per AGENTS.md §0.5 rule 2.

**CPU screen** (batch 9):

- **CoreCell** — one core tile wrapped in `Tooltip` for the keyboard+
  hover-accessible temp/carga/reloj/throttling readout. `unavailable`
  draws a diagonal hatch instead of a value; `throttling` adds a
  bottom-edge status bar — both a pattern, not just a color change.
- **CpuTopologyMap** — a per-core-group grid with a temperature/clock
  toggle. A homogeneous CPU is just one ungrouped group; a hybrid one
  is 2–3 labeled groups — the component has no special-cased branching
  between the two, it renders whatever groups it's given.
- **CpuAdvancedTable** — filterable (by core number/group), sortable
  (click a header), and virtualized with a hand-rolled windowed render
  (track scroll position, render only the visible rows plus overscan,
  pad the rest with spacer elements) — this system ships zero runtime
  dependencies, so no virtualization library.
- **CpuScreen** — composes the two; owns the temperature/clock toggle
  as local view-only state.

**Análisis screen** (batch 10):

- **AnalysisChart** — a hand-rolled SVG chart: every track (temp/clock/
  load/power) stacked inside one `<svg>` so a single pointer-move
  handler draws one shared cursor across all of them. Gaps (`value:
  null`) break the line into separate segments rather than
  interpolating; reduced-quality samples draw with a dashed stroke;
  event bands include a genuine two-color diagonal pattern for a
  `'mixed'` thermal+electrical event (never a blended third color); a
  slim strip below the plot is a drag-to-select zoom/range brush that
  is also a real keyboard slider (arrow keys move the cursor, Home/End
  jump to the extremes, Enter/Space arms then commits a range, Escape
  cancels) with a visible range summary and a hidden tabular
  alternative for screen readers. All of its text — value formatting,
  the slider's accessible name, the range summary sentence, the reset-
  range button label — comes in via props (`valueLabel`, `cursorLabel`,
  `rangeSummaryLabel`, `resetRangeLabel`). **This SVG implementation is
  the definitive production chart, not a placeholder for ECharts** —
  see AGENTS.md's "AnalysisChart: SVG vs. ECharts" (§2, under
  `AnalysisChart`) for the verified performance envelope and point-
  budget guidance.
- **AnalysisScreen** — wraps the chart with `noHistory`/`loading`/
  `ready` states, an optional stale/partial-data `Banner`, and an
  evidence panel that shows a selected event's evidence, then a
  selected range's, then an idle hint — in that priority order, with
  the actual event/range lookup done by the host, never by this
  component.

**Informe screen** (batch 11):

- **ReportScreen** — a diagnosis told as a narrative rather than a
  dashboard: resultado en una frase, qué se observó, impacto estimado,
  evidencias, causas alternativas, qué no puede concluirse,
  recomendaciones, método del impacto, comparación antes/después. Any
  section with empty/undefined content is omitted entirely, not
  rendered as a stub; `impactValue` follows the same "no method, no
  fabricated number" rule as `StatusHero.performance`; the optional
  causal-chain block reuses `CausalRail` under that component's own
  "only with real evidence" rule. The before/after comparison table's
  two column headers are `comparisonBeforeLabel`/`comparisonAfterLabel`
  props, not hardcoded "Antes"/"Después" text.

**Batch 12 — spec-review additions:**

- **CoverageMatrix** — the "Cobertura" table (FR-023): one row per
  magnitude with available (glyph + text), quality (`substitute` gets a
  dashed underline, a pattern not just a color), source and reason;
  footer with the strongest reachable conclusion and the low-level
  access state. Only `installable`/`upgradable` render the install/update action.
  Opened as a panel from `ContextStrip` and embedded in Ajustes →
  Sensores via `SettingsSensorsSection.coverage`.
- **ContextStrip** — the strip above `StatusHero`: CPU + topology,
  power source/plan, collector state (colored dot + text) with sample
  freshness, and "Ver cobertura". `compact` wraps it to two lines.
- **ExportDialog** — one dialog for the three export scopes. Format,
  anonymize, included/excluded field lists (host-computed), estimated
  size and proposed file name. Never shows a path.
- **ImportResultDialog** — progress, success (+ open session) or an
  actionable error, with warnings listed.
- **FirstCloseDialog** — exit vs. "continue in the tray and keep
  measuring"; dismissing persists nothing. **CloseBlockedDialog** —
  close during guided/export/download/install (install has no confirm).
  Both guard against the native `<dialog>` close event re-firing the
  dismiss callback after an explicit choice.
- **WhatsNewCards** — post-update notices over "Ahora", never modal.
- **TechnicalSummary** — monospaced identifier-free summary + Copiar.
- **LicensesScreen** — expandable third-party notices.
- **`BottomBar.menu`** — the 4th "Más" slot with a popup menu, so
  Sesiones / Diagnóstico guiado / Ajustes stay reachable in compact.

**"Ahora" screen + shell:**

- **TitleBar** — the app's own window chrome (icon, name, drag region,
  minimize/maximize-restore/close). See its doc comment for wiring to
  `@tauri-apps/api/window`. The three buttons' accessible names are
  `minimizeLabel`/`maximizeLabel`/`restoreLabel`/`closeLabel` props
  (four, since maximize and restore share one button), not hardcoded
  Spanish `aria-label`s.
- **NavigationItem** — one sidebar row, two densities (`labeled` /
  `icon-only`); the app shell decides which.
- **BottomBar** — compact-width navigation: 3 direct items plus the
  `menu` ("Más") slot. Hard rule: "Ahora" is always present and always
  the first icon.
- **ToolbarButton** — a secondary header action; never a filled
  call-to-action. `compact` drops the label below 700px.
- **StatusHero** — the one prominent conclusion on "Ahora": ring +
  classification tag + optional performance block. Enforces the
  no-method-no-figure rule in code, and paints `severity: 'boost'`
  limitations in the warm tone (within specification).
- **StatWidget** — one signal's tile (temperature, load, clock,
  power), grouped by the consumer in an auto-fit grid.
- **StatusChip** — a pill for one of the 7 `diagnostic_report.classification`
  values, colored from the single `lib/classification.ts` table.
- **CausalRail** — the load→temperature→clock→performance sequence,
  2–4 nodes, evidence-gated. **Has no empty state**: don't render it
  at all when there isn't enough evidence, and don't mount it below a
  980px window width.

Every component's own doc comment (top of its `<script>` block) repeats
the rules specific to it — read the component you're about to use, not
just this file.

## Reproducing the verification harness

Every claim in this file and in `AGENTS.md` about type-checking,
building, and screenshotting this package is reproducible, not just
asserted. `harness/` is a small npm subproject shipped inside this
package, pinned to the exact Svelte version (5.57.0) and tool versions
this system was last verified against:

```sh
cd throttlewatch-design-system/harness
npm install
npm run check    # svelte-check + tsc — 0 errors, 0 warnings expected
npm run build    # vite build — clean production build expected
npm run preview  # serves the build at a local URL
```

`npm run preview` opens a real nav (not just a raw file you have to
know the path to) over every `examples/*.example.svelte` file, with a
dark/light theme toggle. It's the same nav used to take every
screenshot referenced in `AGENTS.md`'s verification section — append
`?view=settings` (or any other id from `harness/src/App.svelte`'s
`views` array) to deep-link one example directly, which is also how a
screenshot script can regenerate any of them.

`harness/` is not part of the design system — don't import from it,
and don't copy it into your app. Copy everything else one level up
(`components/`, `icons/`, `illustrations/`, `lib/`, `tokens/`,
`brand/`) instead; `harness/tsconfig.app.json` reaches up into those
real folders to type-check them in place, so running `npm run check`
here always reflects the actual shipped source, not a snapshot.

## What this package deliberately does NOT include

- No copy/strings (bilingual catalog is the consuming app's job).
- `NavigationItem` / `BottomBar` still just accept an `icon` snippet —
  they don't hardcode a glyph — but `icons/NavIcon.svelte` now ships
  the actual 6 destination glyphs to pass into that slot (see
  Components below), so you no longer need to draw your own for the
  sidebar/BottomBar. `ToolbarButton` / `StatWidget` / `CausalRail`
  still have no bundled icon set — `examples/AhoraScreen.example.svelte`
  shows a working set of line icons in the house style (stroke-width
  2, round caps) you can copy as a starting point for those.
- No crash/usage reporting, no update channel: the product has zero
  network besides the opt-in updater, so no component offers a switch
  for it (gap-analysis.md 1.1 / 1.3).
- No text input / slider — nothing in the spec currently needs free
  text entry, and no screen has asked for a continuous-value control.
  `Select` (batch 4) now covers longer option lists; `Switch`/
  `SegmentedControl` still cover toggles and short exclusive choices.
  Add a text input or slider only once a screen genuinely needs one,
  matching this system's tokens rather than inventing a one-off style.
- No toast component — the spec routes transient alerts through native
  Windows notifications, not in-app toasts. `Banner` (batch 4) is for a
  *persistent* in-page notice instead, which is a different thing
  (doesn't auto-expire, doesn't come from the OS).
- No state management / data fetching — every component is
  presentational and takes plain props.
- No Tauri API calls — `TitleBar`'s callbacks are where you wire
  `@tauri-apps/api/window`.
