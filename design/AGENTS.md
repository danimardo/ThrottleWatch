# AGENTS.md — ThrottleWatch design system

You are an AI coding agent integrating this design system into a Tauri
+ Svelte app. This file is the fast path: exact paths, exact props,
exact rules. `README.md` has the prose explanation if something here
is unclear; `design-system.json` has the same component/prop data as
structured JSON if you'd rather parse than read. Do not invent
props, tokens, or components beyond what is listed here — if a screen
needs something this system doesn't have, say so instead of
approximating it with an unrelated component.

## 0. Scope — what exists and what doesn't

Eleven batches, covering every screen in the spec. Batch 1 (8 components) is the "Ahora" screen and
its navigation chrome. Batch 2 (5 components) is the generic
primitives: `Button`, `Switch`, `SegmentedControl`, `Dialog`,
`OptionRow`. Batch 3 is brand assets: the app icon (`brand/app-icon/`),
6 custom nav glyphs (`icons/NavIcon.svelte`), and 5 onboarding
illustrations (`illustrations/Onboarding*.svelte`). Batch 4 is more
generic primitives, added specifically to avoid raw HTML elements
leaking OS-styled chrome: `Select` (a themeable dropdown, replacing a
native `<select>`), `Tooltip` (keyboard+hover accessible, replacing a
native `title` attribute that fails the spec's own accessibility
requirement), `ProgressBar` (extracted out of `StatusHero`'s own bar
markup — `StatusHero` is now built on it), `Banner`, and `EmptyState`.

Batch 5 is the onboarding slide flow itself: `OnboardingProgress`,
`OnboardingSlide`, `OnboardingFlow` — the state machine that sequences
the 5 existing illustrations with real Atrás/Siguiente/Omitir
navigation, a resumed-session note, and slide 5's passive
hardware-detection states. It is presentational only: `OnboardingFlow`
never runs real detection, it only renders whatever detection status
you pass it (see its prop reference below and rule 10).

Batch 6 is the full "Ajustes" (Settings) screen: `SettingsScreen`, an
exported component (not just an example composition, unlike the
earlier `examples/SettingsSection.example.svelte`, which is now
superseded and kept only as a smaller reference for the primitives it
combines). It composes all 11 sections the app needs — General,
Idioma, Apariencia, Monitorización, Bandeja y notificaciones, Datos y
privacidad, Sensores y cobertura, Diagnóstico, Actualizaciones, Acerca
de y ayuda, Zona de riesgo — entirely from the existing primitives
(`OptionRow`, `Switch`, `SegmentedControl`, `Select`, `Button`,
`Dialog`, `ProgressBar`, `Banner`); none of those primitives were
modified. It is presentational: every current value, option list,
disabled/reason pair, and status (sensor coverage, update progress,
delete/reset progress) is a prop — see its prop reference below and
rule 11.

Batch 7 is the "Sesiones" screen: `SessionCard` (one session row —
active/completed/cancelled/incomplete/imported, with export/delete
actions) and `SessionsScreen` (loading/error/empty/loaded list, one
shared delete-confirmation `Dialog`). Presentational: `status` and the
`sessions` array are entirely host-computed.

Batch 8 is the "Diagnóstico guiado" screen: `GuidedDiagnosticScreen`, a
single component driven by a `phase` prop (12-value union covering
preflight through result/cancelled/error/safety-stop/sensor-lost),
rendering its own fixed phase-stepper and a battery-state banner.
Presentational, same contract as `OnboardingFlow`/`SettingsScreen`: it
never runs the diagnostic itself, only renders whichever phase/reading/
result you pass it.

Batch 9 is the "CPU" screen: `CoreCell` (one core tile, wrapped in
`Tooltip`), `CpuTopologyMap` (per-core-group grid with a temperature/
clock toggle), `CpuAdvancedTable` (filterable, sortable, and
hand-rolled-virtualized — no virtualization library, this system ships
zero runtime dependencies), and `CpuScreen` (composes the two; owns
only the view-only metric-mode toggle as local state).

Batch 10 is the "Análisis" screen: `AnalysisChart` (a hand-rolled SVG
synced multi-track chart — temperature/clock/load/power sharing one
time axis, one hover cursor, gap-aware line segments, reduced-quality
dashed segments, event bands including a diagonal two-color pattern for
a `'mixed'` thermal+electrical event, and a drag-to-select brush/zoom
strip) and `AnalysisScreen` (wraps the chart with a `noHistory`/
`loading`/`ready` status, an optional stale/partial data `Banner`, and
an evidence panel showing `selectedEventEvidence` >
`rangeEvidence` > an idle hint, in that priority order).

Batch 11 is the "Informe" screen: `ReportScreen`, a narrative report
(resultado en una frase, qué se observó, impacto estimado, evidencias,
causas alternativas, qué no puede concluirse, recomendaciones, método
del impacto, comparación antes/después) rather than a dashboard. Every
section is omitted entirely (not padded with a stub heading) when its
content prop is empty/undefined; the optional `causalChain` prop maps
straight to `CausalRail` and follows that component's own "only with a
real 2–4 node chain" rule.

Batch 12 (2026-09-18, spec review — `specs/001-cpu-thermal-diagnostics/
gap-analysis.md` §10, tasks T118–T129) adds `CoverageMatrix`,
`ContextStrip`, `ExportDialog`, `ImportResultDialog`, `FirstCloseDialog`,
`CloseBlockedDialog`, `WhatsNewCards`, `TechnicalSummary`,
`LicensesScreen`, the `menu` prop on `BottomBar`, `size="wide"` on
`Dialog`, `lib/access.ts` (`AdvancedAccessState`) and contract changes to
`SettingsScreen`, `OnboardingFlow`, `GuidedDiagnosticScreen`,
`SessionCard`/`SessionsScreen` and `ReportScreen` — see §2 below and
rules 19–22.

All twelve batches are now built — every screen the spec describes has
a dedicated component. There is no remaining "not yet built" screen;
if a task asks for something none of the props below cover, that's a
genuinely new requirement, not a gap in this inventory.

## 0.5 Non-negotiable rules (violating these breaks the product's own constitution)

1. **No invented performance figure.** Since the 2026-09-18 engine
   review there is no "available performance %" at all. The only
   figures are (a) the **cooling potential** from the host's
   power-headroom method (`+10–20 %`, level-A equipment only, spec
   FR-013) and (b) the **measured** guided-test result. `StatusHero`'s
   `performance` prop is optional for exactly this reason — without
   one of those two sources, omit it (or pass `null`); never compute a
   clock ratio or guess a number to fill the slot.
2. **No copy lives in this package.** Every visible string is a prop —
   and so is every *accessible* string: `aria-label`, `title`, `alt`,
   placeholder text, and any other text exposed only to assistive tech
   or a tooltip. `Banner`'s and `TitleBar`'s dismiss/window-control
   `aria-label`s, `GuidedDiagnosticScreen`'s stepper captions, and
   `AnalysisChart`'s value/range formatting are all props for this
   reason — an audit that only checks visible `<span>` text and misses
   an `aria-label="Cerrar"` still leaves hardcoded Spanish in the
   package. The app is bilingual ES/EN from day one — pull strings from
   the app's own i18n catalog, never hardcode Spanish or English inside
   a call site you're generating for reuse. `examples/*.example.svelte`
   are the one exception: they hardcode Spanish for demo brevity, since
   a real app supplies these same props from its own ES/EN catalog.
3. **`CausalRail` has no empty state.** If evidence doesn't support a
   causal sequence, don't render the component — don't pass an empty
   or padded `nodes` array. Also don't mount it when the window/shell
   width is below 980px.
4. **`BottomBar`: "Ahora" is always item 0.** If a compact bar needs a
   5th destination, remove a different one — never reorder Ahora out
   of first position or drop it.
5. **`accent-blue` is never a status color.** Use it for navigation,
   focus, toolbar actions, and neutral figures only. Thermal/power/
   normal/unknown states use their own `status-*` token via
   `Classification` / `Tone`, never blue.
6. **Elevation is the glass material, not ad-hoc shadows.** A surface
   is either glass (uses `--glass-bg*`, `--glass-border`,
   `--glass-shadow*` — see rules 24–26) or flat with a 1px `hairline`.
   Don't invent a `--shadow-*` token or a one-off `box-shadow`.
7. **Don't add a web font.** `--font-sans` already resolves to Segoe
   UI on Windows; do not import Google Fonts or bundle a `.woff2`.
8. **`Button`'s `destructive` variant is only for a `Dialog`'s confirm
   button**, never for the row/entry-point control that opens that
   dialog (use `variant="secondary"` there, inside a labeled "risk
   zone" section). This keeps `status-thermal` meaning "thermal
   limitation" everywhere it appears outside an active confirmation.
9. **Don't draw new onboarding illustrations in a different visual
   style** (Corporate Memphis / human figures) without the user
   explicitly asking for that specific departure — the 5 shipped
   illustrations, the app icon and `NavIcon` all share one abstract,
   geometric vocabulary on purpose. A screen with 5 flat-mascot slides
   next to hairline-bordered gauges elsewhere reads as two different
   apps stitched together.
10. **`OnboardingFlow` has no business logic and never will.** Its
    slide-5 detection state (`'detecting' | 'complete' | 'partial'`,
    `advancedAccessAvailable`) is driven entirely by the `steps[4]`
    prop object — it does not read hardware, call Tauri, or persist
    anything. Wire real detection, retry, and "request advanced
    access" behavior in the host app and feed the *result* in; don't
    add IPC calls or timers inside this component to simulate it.
11. **`SettingsScreen` has no business logic and never will**, same
    contract as rule 10. Every disabled state, dependency reason,
    sensor-coverage status, and update-flow status is a prop the host
    computes and passes in — the component does not decide *why*
    something is disabled or *what* a rechecked/retried/reset action
    finds. Its only local state is UI-only: whether the "eliminar
    datos" and "restablecer" confirmation dialogs are open. Calling
    `onDeleteAllData`/`onReset` starts the real action in the host app;
    `SettingsScreen` never touches storage or Tauri itself.
12. **`SessionsScreen`/`GuidedDiagnosticScreen`/`CpuScreen`/
    `AnalysisScreen`/`ReportScreen` have no business logic either**,
    same contract as rules 10–11. None of them fetch, sort, filter,
    classify, or compute anything from raw sensor data — `status`,
    `phase`, `groups`/`rows`, `tracks`/`events`, and every narrative
    string are host-computed props. The only local state any of them
    owns is view-only UI state that never leaves the component: which
    delete-confirmation dialog is open (`SessionsScreen`), the
    temperature/clock display toggle (`CpuScreen`), and a table's
    filter/sort/scroll position (`CpuAdvancedTable`).
13. **`CpuAdvancedTable`'s virtualization and `AnalysisChart`'s chart
    are hand-rolled, not a dependency.** This system ships zero runtime
    dependencies — don't reach for a virtualization library or a
    charting library to extend either of these; extend the existing
    hand-rolled approach (windowed `scrollTop`/spacer-div rendering for
    the table, one shared `<svg>` with per-track `<g>` offsets for the
    chart) instead.
14. **A `null` sample in an `AnalysisTrack.points` array is a real
    gap, never bridged by interpolation.** `AnalysisChart` breaks the
    line into separate path segments around it — don't pass `0` or a
    carried-forward value to paper over missing data, and don't "fix"
    the chart by connecting the segments.
15. **`ReportScreen` never fabricates `impactValue`.** Same rule as
    `StatusHero.performance` (rule 1) applied to the Informe screen:
    when no method applies (power limit unknown, no guided result) or
    confidence is too low, omit
    `impactValue` and supply `impactUnavailableTitle`/
    `impactUnavailableReason` instead — never compute a placeholder
    percentage to fill the slot.
16. **A `ReportScreen` section with empty/undefined content is
    omitted, never padded.** Don't pass `[]`/`undefined` and then wonder
    why "Causas alternativas" disappeared — that's the component
    hiding an empty section on purpose, matching the product's own
    "a missing value is stated explicitly, never a hidden placeholder
    row" rule applied to whole sections instead of single values.
17. **An ARIA role must match real behavior — never assign one "for
    the semantics" without the interaction to back it up.** `role`,
    `aria-valuenow`/`aria-valuetext`, and similar attributes describe
    what a control actually does; if you extend a component with a new
    interactive affordance, wire the real keyboard/value behavior
    first and only then reach for the role that names it. Two examples
    already fixed in this package: `AnalysisChart`'s `.brush` is
    `role="slider"` only because arrow keys really move
    `aria-valuenow` (see its keyboard model above) — before that
    existed, assigning the role was a lie a screen reader would repeat
    to the user; and its outer `<svg class="plot">` is `role="group"`,
    not `role="img"`, because it contains real focusable
    `role="button"` event bands that `role="img"` would hide from the
    accessibility tree.
18. **`AnalysisChart` is the definitive production chart, not a
    placeholder for ECharts.** See "AnalysisChart: SVG vs. ECharts"
    under §2 for the decision, the verified performance envelope, and
    what to do if a future track ever needs to exceed it.
19. **No network-shaped controls.** There is no crash/usage-report
    switch, no "send diagnostic" link and no update channel selector
    anywhere in this package, and none may be added: the product's
    only network is the opt-in updater (spec FR-029/FR-058). "Copiar
    resumen técnico" (`TechnicalSummary`) is the supported way to ask
    for help.
20. **Three update gestures.** `SettingsUpdatesSection.status` goes
    `available` → (Descargar) → `downloading` → `verified` → (Instalar)
    → `installing`. Never render an "install" button in `available`,
    never auto-advance, and always pass `installDisabledReason` when
    `installDisabled` is true (a guided test, export, import or wipe is
    running).
21. **`advancedAccess` is the 5-value enum from `lib/access.ts`, never
    a boolean.** Only `installable`/`upgradable` render "Instalar/Actualizar acceso
    avanzado"; the host (Rust) decides `not_needed` from the coverage,
    the component never infers it.
22. **The close action is tri-state.** `SettingsGeneralSection.closeAction`
    is `'unset' | 'exit' | 'tray'`; while `unset` no segment is selected
    and `closeActionUndecidedText` is shown. `FirstCloseDialog.onDismiss`
    persists nothing. Choosing tray also turns on background monitoring
    (host does it; `trayNote` explains it).
23. **Global notices are shell compositions of `Banner`, mounted
    directly under `TitleBar`, in every screen and every width**: a
    running guided test (with Detener) and a degraded/stopped collector
    (Reintentar / Ver resumen técnico). Don't reinvent them per screen.
24. **Glass alpha is not a knob.** Use `--glass-bg` (cards),
    `--glass-bg-strong` (floating chrome) or `--glass-bg-subtle`
    (tiles on glass); never lower the alphas or raise the blur in a
    component — legibility of text on glass is a product requirement.
    Status colours, the thermal ring, event bands and reduced-quality
    patterns are never glass.
25. **The shell owns the ambient background and the quality attributes.**
    Mount `.tw-ambient` once on the root container, keep screen roots
    transparent, and mirror preferences with `applyGlassLevel()` /
    `applyMotionLevel()` from `tokens/tokens.ts`. Components must look
    right at every `data-glass` level and without `backdrop-filter`.
26. **Animate transitions and state changes, never rest.** Use the
    `--motion-*` tokens and the `.tw-enter`/`.tw-pop` utilities; pass
    `enterIndex`/`--tw-i` for staggers. The only continuous motion at
    rest is `StatusHero`'s halo. Every animation must collapse to
    instant under reduced motion (it does automatically via tokens.css
    if you use the tokens and don't hardcode durations). Decorative
    motion is `aria-hidden` and never the only signal.
27. **Never underline on hover/focus, never a hand cursor on a
    button.** ThrottleWatch reads as a native Windows instrument, not
    a web page. Every action — a `Button`, a link-styled row (chevron
    `OptionRow`, "Ver informe", "Ver cobertura"), a chip, a tab — uses
    `cursor: default` and communicates hover/active/focus with color,
    weight and the existing focus ring, never with
    `text-decoration: underline` appearing on `:hover`. `cursor:
    pointer` is reserved for genuine graphical affordances inside
    `AnalysisChart`'s plotting surface (crosshair scrubbing, a
    clickable event band) — never for a button, tab or link-styled
    text. In-app navigation and actions use `<button>`, never `<a
    href>`; see tokens.css's own comment above the type-style rules
    for the full rationale.
28. **Nine classifications and a severity, from the engine only.**
    `platform_limited` ("limitada por el equipo": manufacturer
    thermal management lowering the power limit, or an external
    PROCHOT) has its own `device` icon and `status-warm` colour; never
    render it as thermal red or as power purple. A limitation's
    severity (`boost` / `below_base`) comes from the host; `boost` is
    within specification and must never be styled or worded as a
    problem. In `AnalysisChart`, `platform` events are warm striped
    bands and `info` events (e.g. the end of the power-turbo window)
    are thin dashed markers — never bands, never red.

## 1. Install

Copy `throttlewatch-design-system/` as-is into the app, typically at
`src/lib/design-system/` (SvelteKit) or `src/design-system/` (plain
Svelte + Vite). Nothing to `npm install` — zero runtime dependencies
beyond Svelte 5.

Import once near the app root:

```ts
import '$lib/design-system/tokens/tokens.css';
```

Toggle theme anywhere in the app:

```ts
document.documentElement.dataset.theme = 'dark' | 'light';
```

Import components:

```ts
import {
  TitleBar, NavigationItem, BottomBar, ToolbarButton,
  StatusHero, StatWidget, StatusChip, CausalRail,
  CLASSIFICATION_META, TONE_TOKENS, BREAKPOINTS,
  createWidthTracker
} from '$lib/design-system/components';
```

`examples/AhoraScreen.example.svelte` is a complete working
composition — read it before assembling a screen from scratch; it is
the reference for how the shell (TitleBar + sidebar/BottomBar swap +
StatusHero + StatWidget grid + CausalRail) fits together and reflows.

## 2. Component prop reference

All components are Svelte 5 runes components (`$props()`). Icon slots
are Svelte 5 **snippets** (`Snippet` type from `'svelte'`), passed like:

```svelte
{#snippet flameIcon()}<svg ...>...</svg>{/snippet}
<StatWidget icon={flameIcon} ... />
```

### Button
| prop | type | required | notes |
|---|---|---|---|
| `label` | `string` | yes | |
| `variant` | `'primary' \| 'secondary' \| 'destructive'` | no (`'primary'`) | see rule 8 above for `destructive` |
| `icon` | `Snippet` | no | leading icon |
| `disabled` | `boolean` | no (`false`) | |
| `type` | `'button' \| 'submit'` | no (`'button'`) | |
| `onclick` | `() => void` | no | |

### Switch
| prop | type | required | notes |
|---|---|---|---|
| `checked` | `boolean` (bindable) | no (`false`) | `bind:checked={...}` |
| `label` | `string` | yes | used as `aria-label` |
| `disabled` | `boolean` | no (`false`) | pair with `OptionRow`'s `disabledReason`, don't just disable silently |
| `onchange` | `(checked: boolean) => void` | no | |

### SegmentedControl
| prop | type | required | notes |
|---|---|---|---|
| `options` | `{ value: string; label: string }[]` | yes | 2–5 items in practice |
| `value` | `string` (bindable) | no (first option) | `bind:value={...}` |
| `label` | `string` | yes | group `aria-label` |
| `disabled` | `boolean` | no (`false`) | |
| `onchange` | `(value: string) => void` | no | |

### Dialog
| prop | type | required | notes |
|---|---|---|---|
| `open` | `boolean` (bindable) | no (`false`) | `bind:open={...}` |
| `size` | `'default' \| 'wide'` | no (`'default'`) | `wide` = 560px for dialogs with lists (ExportDialog) |
| `title` | `string` | yes | |
| `description` | `string` | no | simple case; use `body` snippet for anything richer |
| `body` | `Snippet` | no | overrides `description` when present |
| `tone` | `'default' \| 'warning'` | no (`'default'`) | adds a warning mark next to the title; does not itself pick the Button variant |
| `actions` | `Snippet` | yes | compose with `Button` — see rule 8 |
| `onclose` | `() => void` | no | fires on Escape/backdrop dismiss too, not only a button click |

### NavIcon (`icons/NavIcon.svelte`)
| prop | type | required | notes |
|---|---|---|---|
| `kind` | `'now' \| 'analysis' \| 'cpu' \| 'sessions' \| 'guided' \| 'settings'` | yes | one per sidebar/BottomBar destination |

Not a `Snippet` itself — wrap it in one: `{#snippet icon()}<NavIcon kind="now" />{/snippet}`, then pass `icon` to `NavigationItem`/`BottomBar` as usual. Colored with `currentColor`, so it inherits the active-state accent-blue automatically.

### Onboarding illustrations (`illustrations/Onboarding*.svelte`)
Five components, one per onboarding slide, same prop shape on all five:
| prop | type | required | notes |
|---|---|---|---|
| `width` | `number \| string` | no (`'100%'`) | |
| `height` | `number \| string` | no (`'100%'`) | |

`OnboardingWelcome` (slide 1), `OnboardingSignals` (slide 2),
`OnboardingConclusions` (slide 3), `OnboardingPrivacy` (slide 4),
`OnboardingThisComputer` (slide 5). No other props — they're pure SVG
artwork, viewBox `0 0 280 220`, colored entirely with this system's CSS
tokens (no hardcoded hex). Do not add a 6th illustration in a
different visual style without checking rule 9 above first.

### OptionRow
| prop | type | required | notes |
|---|---|---|---|
| `label` | `string` | yes | |
| `description` | `string` | no | hidden when `disabled` + `disabledReason` are both set |
| `disabled` | `boolean` | no (`false`) | dims the row; does NOT disable your `control` snippet's contents — set that yourself, matching |
| `disabledReason` | `string` | no | shown instead of `description` when `disabled` |
| `control` | `Snippet` | yes | the trailing Switch/SegmentedControl/Button |

### CoverageMatrix
| prop | type | required | notes |
|---|---|---|---|
| `rows` | `CoverageRow[]` | yes | `{ id, label, available, quality?: 'direct'\|'derived'\|'substitute', qualityLabel?, sourceLabel?, reasonLabel? }` |
| `columnLabels` | `{ magnitude, available, quality, source, reason }` | yes | |
| `availableLabel` / `unavailableLabel` | `string` | yes | |
| `maxConfidenceLabel` | `string` | yes | full sentence, host-computed |
| `accessState` | `AdvancedAccessState` | yes | |
| `accessLabel` | `string` | yes | sentence describing the state |
| `requestAccessLabel` / `onRequestAccess` | `string` / `() => void` | no | rendered only for `installable`/`upgradable` |
| `accessRetryLabel` / `onAccessRetry` | `string` / `() => void` | no | rendered only for `error` |
| `recheckLabel` / `onRecheck` / `recheckDisabled` | `string` / `() => void` / `boolean` | yes/yes/no | |
| `copySummaryLabel` / `onCopySummary` | `string` / `() => void` | no | |
| `title` | `string` | no | |

Collapses to stacked cards below 480px of its own container width.

### ContextStrip
| prop | type | required | notes |
|---|---|---|---|
| `cpuLabel` | `string` | yes | |
| `topologyLabel` | `string` | no | e.g. `"6P + 8E + 2LP"` |
| `powerLabel` | `string` | no | e.g. `"Batería 64 % · Equilibrado"` |
| `collectorState` | `'fresh' \| 'stale' \| 'disconnected' \| 'starting'` | yes | dot color + text; host decides (stale after > 5 s) |
| `collectorLabel` | `string` | yes | e.g. `"Conectado · hace 1 s"` |
| `coverageActionLabel` / `onCoverage` / `coverageIcon` | `string` / `() => void` / `Snippet` | no | "Ver cobertura" |
| `compact` | `boolean` | no (`false`) | wraps to two lines |

### ExportDialog
| prop | type | required | notes |
|---|---|---|---|
| `open` | `boolean` (bindable) | no | |
| `title` / `scopeLabel` | `string` | yes | scope = session / report / range, described in words |
| `formatLabel` / `format` / `formatOptions` / `onFormatChange` | … | yes | `'csv' \| 'json'` |
| `anonymizeLabel` / `anonymizeDescription` / `anonymize` / `onAnonymizeChange` | … | yes (description no) | preset from `privacy.anonymize_exports` |
| `includedTitle` / `includedFields` / `excludedTitle` / `excludedFields` | `string` / `string[]` | yes | host recomputes on every change (`preview_export`) |
| `sizeLabel` / `fileNameLabel` | `string` | no | |
| `cancelLabel` / `onCancel` / `confirmLabel` / `onConfirm` / `confirmDisabled` | … | yes/yes/yes/yes/no | `onCancel` is not re-fired after `onConfirm` |

### ImportResultDialog
| prop | type | required | notes |
|---|---|---|---|
| `open` | `boolean` (bindable) | no | |
| `title` | `string` | yes | |
| `status` | `'reading' \| 'validating' \| 'migrating' \| 'success' \| 'error'` | yes | |
| `progressLabel` / `description` | `string` | no | |
| `warningsTitle` / `warnings` | `string` / `string[]` | no | listed, never hidden |
| `closeLabel` / `onClose` | `string` / `() => void` | yes | |
| `openSessionLabel` / `onOpenSession` | `string` / `() => void` | no | only in `success` |

### FirstCloseDialog
| prop | type | required | notes |
|---|---|---|---|
| `open` | `boolean` (bindable) | no | |
| `title` / `description` / `exitLabel` / `trayLabel` | `string` | yes | |
| `trayNote` / `settingsHint` | `string` | no | "activa la monitorización" / "cámbialo en Ajustes" |
| `onExit` / `onTray` / `onDismiss` | `() => void` | yes | `onDismiss` = Escape/backdrop; not fired after a choice |

### CloseBlockedDialog
| prop | type | required | notes |
|---|---|---|---|
| `open` | `boolean` (bindable) | no | |
| `reason` | `'guided' \| 'export' \| 'download' \| 'install'` | yes | picks tone and the destructive confirm for `guided` |
| `title` / `description` / `cancelLabel` / `onCancel` | … | yes | |
| `confirmLabel` / `onConfirm` | `string` / `() => void` | no | omit for `install` (cannot close) |

### WhatsNewCards
| prop | type | required | notes |
|---|---|---|---|
| `cards` | `WhatsNewCard[]` | yes | `{ id, title, body, actionLabel?, onAction? }`; renders nothing when empty |
| `title` | `string` | no | |
| `dismissLabel` / `onDismiss` | `string` / `(id) => void` | yes | host removes the card |
| `dismissAllLabel` / `onDismissAll` | `string` / `() => void` | no | shown with > 1 card |

### TechnicalSummary
| prop | type | required | notes |
|---|---|---|---|
| `text` | `string` | yes | already identifier-free |
| `copyLabel` / `onCopy` / `regionLabel` | `string` / `() => void` / `string` | yes | host writes the clipboard |
| `copiedLabel` / `copied` | `string` / `boolean` | no | flip `copied` briefly to confirm |
| `title` / `note` | `string` | no | |

### LicensesScreen
| prop | type | required | notes |
|---|---|---|---|
| `title` | `string` | yes | |
| `intro` | `string` | no | |
| `entries` | `LicenseEntry[]` | yes | `{ id, name, version?, license, text }` |

### TitleBar
| prop | type | required | notes |
|---|---|---|---|
| `title` | `string` | yes | app name shown next to the mark |
| `maximized` | `boolean` | no (default `false`) | swaps the maximize/restore icon |
| `onMinimize` | `() => void` | yes | wire to `getCurrentWindow().minimize()` |
| `onMaximizeToggle` | `() => void` | yes | wire to `getCurrentWindow().toggleMaximize()`; also fires on drag-region double-click |
| `onClose` | `() => void` | yes | wire to `getCurrentWindow().close()` |
| `icon` | `Snippet` | no | optional app mark before the title |
| `minimizeLabel` | `string` | yes | `aria-label` for the minimize button — no copy lives in this package (rule 2) |
| `maximizeLabel` | `string` | yes | `aria-label` for the maximize button when `maximized` is `false` |
| `restoreLabel` | `string` | yes | `aria-label` for the same button when `maximized` is `true` |
| `closeLabel` | `string` | yes | `aria-label` for the close button |

### NavigationItem
| prop | type | required | notes |
|---|---|---|---|
| `icon` | `Snippet` | yes | |
| `label` | `string` | yes | |
| `active` | `boolean` | no (`false`) | accent-blue only, never a status color |
| `density` | `'labeled' \| 'icon-only'` | no (`'labeled'`) | app shell picks this from width tier |
| `onclick` | `() => void` | no | |

### BottomBar
| prop | type | required | notes |
|---|---|---|---|
| `items` | `BottomBarItem[]` | yes | `{ id, label, icon: Snippet, active?, onclick? }`. First item must be "Ahora". Max 3 when `menu` is passed (4 otherwise). |
| `menu` | `BottomBarMenu` | no | `{ id, label, icon, menuLabel, items: BottomBarMenuItem[] }` — the "Más" slot: opens a popup `role="menu"` above the bar (Escape closes, focus returns). The slot is highlighted while any of its items is active. Product default: Sesiones, Diagnóstico guiado, Ajustes. |

### ToolbarButton
| prop | type | required | notes |
|---|---|---|---|
| `icon` | `Snippet` | yes | |
| `label` | `string` | yes | used as visible text or `aria-label` |
| `compact` | `boolean` | no (`false`) | true below 700px shell width: hides label, keeps `aria-label` |
| `onclick` | `() => void` | no | |
| `disabled` | `boolean` | no (`false`) | never use this component for a destructive action |

### StatusHero
| prop | type | required | notes |
|---|---|---|---|
| `ringValue` | `string` | yes | e.g. `"98°"` |
| `ringCaption` | `string` | yes | e.g. `"TjMax 100°"` |
| `ringPercent` | `number` (0–100) | yes | fraction of ring circumference drawn |
| `classification` | `Classification` | yes | one of the 7 values, see §3 |
| `classificationLabel` | `string` | yes | already-translated tag text |
| `evidenceLine` | `string` | yes | e.g. `"4 °C hasta el límite · confianza alta · observado 3 min 42 s"` |
| `performance` | `{ label, rangeText, percent? } \| null` | no | cooling potential ("Enfriar mejor", "+10–20 %"); **omit/null without the power-headroom method** — do not fabricate. Omit `percent` for a range (no bar) |
| `severity` | `'boost' \| 'below_base'` | no | `boost` paints thermal-family classifications in `status-warm` (within spec); `below_base` keeps their colour |
| `noPotentialText` | `string` | no | override the fallback shown when `performance` is absent |

Row layout above a 900px *component* width, column below — automatic
via CSS container query, not a prop.

### StatWidget
| prop | type | required | notes |
|---|---|---|---|
| `icon` | `Snippet` | yes | |
| `tone` | `Tone` (`'accent'\|'thermal'\|'warm'\|'normal'\|'power'\|'unknown'`) | yes | pick by what THIS reading shows now, not a fixed per-metric color |
| `label` | `string` | yes | |
| `value` | `string` | yes | |
| `unit` | `string` | no | |
| `footnote` | `string` | yes | quality/derivation detail, or the reason a value is missing |

Group several in a container with
`grid-template-columns: repeat(auto-fit, minmax(150px, 1fr))`.

### StatusChip
| prop | type | required | notes |
|---|---|---|---|
| `classification` | `Classification` | yes | drives color+icon from `CLASSIFICATION_META` |
| `label` | `string` | yes | already-translated text |

Never place an `indeterminate` chip next to a `normal` chip in the
same context without a clear indication of which is the active state.

### CausalRail
| prop | type | required | notes |
|---|---|---|---|
| `nodes` | `CausalNode[]` | yes | 2–4 items: `{ id, label, value, tone: Tone, icon: Snippet }`. Dev console warns outside 2–4. |

Don't render the component at all with fewer than 2 supportable nodes.
Connectors are always neutral — never pass a connector color.

### Select
| prop | type | required | notes |
|---|---|---|---|
| `options` | `{ value: string; label: string }[]` | yes | more items than comfortably fit as `SegmentedControl` segments |
| `value` | `string` (bindable) | no (first option) | `bind:value={...}` |
| `label` | `string` | yes | accessible name for trigger + listbox |
| `disabled` | `boolean` | no (`false`) | |
| `onchange` | `(value: string) => void` | no | |

Custom listbox (not a native `<select>`) so the popup stays themeable
in both dark/light instead of an OS-painted dropdown. Keyboard:
ArrowUp/ArrowDown move, Enter/Space choose, Escape closes and returns
focus to the trigger. Reach for `SegmentedControl` first — this is for
when that option range (2–5) doesn't fit.

### Tooltip
| prop | type | required | notes |
|---|---|---|---|
| `label` | `string` | no | simple one-line case |
| `body` | `Snippet` | no | overrides `label` for richer content |
| `placement` | `'top' \| 'bottom' \| 'left' \| 'right'` | no (`'top'`) | |
| `children` | `Snippet<[{ describedBy: string \| undefined }]>` | yes | renders YOUR trigger element; wire `aria-describedby={describedBy}` on it yourself |

Does not render its own trigger (button/tile/etc. is the consumer's) —
it only supplies the show/hide behavior and the `describedBy` id. Shows
on hover AND keyboard focus, and stays open while the pointer is over
the tooltip panel (short grace-period hide, cancelled on re-entry).
Escape closes it. Use this instead of a native `title` attribute
whenever the spec requires a keyboard-reachable tooltip that survives
the pointer moving onto it (the CPU core tiles' temp/carga/reloj/
throttling tooltip is the motivating case).

### ProgressBar
| prop | type | required | notes |
|---|---|---|---|
| `percent` | `number` (0–100) | no (`0`) | ignored when `indeterminate` |
| `tone` | `Tone` | no (`'accent'`) | reuses the shared `Tone` union — see §3 |
| `indeterminate` | `boolean` | no (`false`) | "in progress, no known percentage yet" — never fake a percent to avoid this |
| `label` | `string` | no | `aria-label` |

The 4px hairline-bounded linear track used everywhere progress is
shown. `StatusHero`'s performance bar is now this component internally
— don't reintroduce a second bespoke bar markup elsewhere.

### Banner
| prop | type | required | notes |
|---|---|---|---|
| `tone` | `'info' \| 'warning' \| 'critical'` | no (`'info'`) | a different semantic domain from `Classification` — see notes below |
| `title` | `string` | yes | |
| `description` | `string` | no | |
| `action` | `Snippet` | no | e.g. a `secondary` `Button` |
| `onDismiss` | `() => void` | no | renders a close (X) button when present |
| `dismissLabel` | `string` | no, but required whenever `onDismiss` is set | `aria-label` for the close (X) button — no copy lives in this package (rule 2) |

Persistent in-page notice (connection failure, low-power mode,
update available) — not a toast (doesn't auto-expire), not a `Dialog`
(doesn't block the screen). Deliberately does NOT use `StatusIcon` —
that icon's 6 glyphs are the closed set for
`diagnostic_report.classification` only; `Banner` draws its own 3
local shapes so the two vocabularies never blur together.

### EmptyState
| prop | type | required | notes |
|---|---|---|---|
| `icon` | `Snippet` | no | consumer-supplied, e.g. a `NavIcon`/`StatusIcon` |
| `title` | `string` | yes | |
| `description` | `string` | no | |
| `action` | `Snippet` | no | |

Centered "nothing here yet" placeholder. No card/border by default —
it sits directly on the surface it occupies.

### OnboardingProgress
| prop | type | required | notes |
|---|---|---|---|
| `stepCount` | `number` | yes | |
| `currentStep` | `number` (0-based) | yes | |
| `label` | `string` | no | `aria-label`/`aria-valuetext`; falls back to plain digits (`"2/5"`) when omitted |

### OnboardingSlide
| prop | type | required | notes |
|---|---|---|---|
| `illustration` | `Snippet` | yes | |
| `title` | `string` | yes | |
| `body` | `string` | yes | |
| `stepIndex` | `number` (0-based) | yes | |
| `stepCount` | `number` | yes | |
| `progressLabel` | `string` | no | passed through to `OnboardingProgress`'s `label` |
| `note` | `string` | no | small note above the title (used for the "resumed" case) |
| `extra` | `Snippet` | no | step-specific content below the body (slide 5's detection block) |
| `onBack` | `() => void` | no | omit to hide the "Atrás" button entirely (never render it disabled) |
| `backLabel` | `string` | yes | always pass it; only rendered when `onBack` is set |
| `onSkip` | `() => void` | no | omit to hide "Omitir" |
| `skipLabel` | `string` | yes | same pattern as `backLabel` |
| `onNext` | `() => void` | yes | |
| `nextLabel` | `string` | yes | |

Use directly only for a custom flow that doesn't fit `OnboardingFlow`'s
fixed 5-slide shape — normally you want `OnboardingFlow` itself.

### OnboardingFlow
| prop | type | required | notes |
|---|---|---|---|
| `steps` | `[Content, Content, Content, Content, DetectionContent]` (exactly 5, fixed order) | yes | `Content = { title, body }`; the 5th adds detection fields, see below |
| `initialStep` | `number` (0-based) | no (`0`) | start on this slide — a resumed session |
| `resumedNote` | `string` | no | shown once, only on the slide `initialStep` points to |
| `backLabel` | `string` | yes | |
| `nextLabel` | `string` | yes | used on slides 1–4 |
| `finishLabel` | `string` | yes | replaces `nextLabel` on slide 5 (e.g. "Empezar") |
| `onSkip` | `() => void` | no | omit to hide "Omitir" everywhere; shown on slides 1–4 only (slide 5 has nothing left to skip) |
| `skipLabel` | `string` | no | required in practice if `onSkip` is set |
| `onFinish` | `() => void` | yes | fires when the primary button is pressed on slide 5 |
| `onStepChange` | `(step: number) => void` | no | host callback after moving between slides; receives a 0-based index |
| `progressLabel` | `(step: number, count: number) => string` | no | `step`/`count` are 1-based here (for display); falls back to plain digits |

`DetectionContent` (slide 5's `steps[4]`, extends the plain `{ title, body }` shape):
| field | type | notes |
|---|---|---|
| `status` | `'detecting' \| 'complete' \| 'partial'` | drives which block renders — see rule 10, this is never computed inside the component |
| `detectingLabel` | `string?` | shown next to the indeterminate `ProgressBar` while detecting |
| `coverageTitle` | `string?` | e.g. "Cobertura completa" / "Cobertura parcial" |
| `coverageDescription` | `string?` | what's covered, or what's missing when partial |
| `advancedAccess` | `AdvancedAccessState?` | `installable`/`upgradable` → info `Banner` + request button; `denied` → warning `Banner`; `error` → critical `Banner` + retry; `available`/`not_needed` → quiet caption (rule 21) |
| `advancedAccessNote` | `string?` | the banner/caption text |
| `onRetry` / `retryLabel` | `(() => void)?` / `string?` | action button inside the `partial` warning `Banner` |
| `onRequestAdvancedAccess` / `requestAccessLabel` | `(() => void)?` / `string?` | action button inside the `installable`/`upgradable` `Banner` |
| `onAccessRetry` / `accessRetryLabel` | `(() => void)?` / `string?` | action button inside the `error` `Banner` |

### SettingsScreen
Eleven required section props, each a plain object — every field
inside them is documented in `SettingsScreen.svelte`'s own exported
interfaces (`SettingsGeneralSection`, `SettingsLanguageSection`, …,
`SettingsRiskZoneSection`, all re-exported from `components/index.ts`).
No section has a default: pass all 11, in full, every render. The
shape follows the 2026-09-18 spec review (rules 19–22): tri-state
`closeAction`, no crash-report switch, no update channel, a `verified`
update state with separate Descargar/Instalar, per-section folded
"Avanzado" blocks (`advancedLabel`; fold state is view-only), motion,
on-battery, storage usage, Exportar…, Repetir introducción, and two
optional snippets — `sensors.coverage` (a `CoverageMatrix`) and
`about.technicalSummary` (a `TechnicalSummary`).

| prop | type | notes |
|---|---|---|
| `general` | `SettingsGeneralSection` | `closeAction: 'unset'\|'exit'\|'tray'` as a `SegmentedControl` (none selected while unset + `closeActionUndecidedText`), iniciar con Windows, iniciar oculto (`startHiddenDisabled`/`Reason`) |
| `language` | `SettingsLanguageSection` | one `Select` row |
| `appearance` | `SettingsAppearanceSection` | tema + movimiento (two `SegmentedControl` rows) |
| `monitoring` | `SettingsMonitoringSection` | perfil (`SegmentedControl`), en batería (`SegmentedControl`), folded Avanzado: intervalo exacto (`Select`, `intervalDisabled`/`Reason`) + detalle por núcleo (`Switch`) |
| `tray` | `SettingsTraySection` | background monitoring, notifications, probar notificación (`testNotificationDisabled`/`Reason`), folded Avanzado: periodo de silencio (enable + two hour `Select`s) + read-only `alertRulesNote` |
| `privacy` | `SettingsPrivacySection` | retención (`Select`, exactly session/1d/7d/30d), espacio usado (read-only), aviso de base de datos anterior dañada (FR-075: `corruptBackupNotice` optional — row hidden unless the app just recovered from one, `corruptBackupRowLabel`/`corruptBackupExportLabel`/`onExportCorruptBackup` always required), anonimizar exportaciones, Exportar… (`exportDisabled`/`Reason`), "eliminar todos mis datos" (internal confirm `Dialog`, `deleteStatus` drives `ProgressBar`/`Banner`). **No crash-report switch** (rule 19) |
| `sensors` | `SettingsSensorsSection` | `'detecting'\|'complete'\|'partial'` + `advancedAccess: AdvancedAccessState` (rule 21) + optional `coverage` Snippet that replaces the short line with an embedded `CoverageMatrix`; the `'complete'` state draws its own local checkmark, not `StatusIcon` |
| `diagnostics` | `SettingsDiagnosticsSection` | duración short/standard/long (`SegmentedControl`), exigir CA, avisar al terminar (`notifyDisabled`/`Reason`), read-only `safetyLimits[]` block |
| `updates` | `SettingsUpdatesSection` | `status: 'idle'\|'checking'\|'upToDate'\|'available'\|'downloading'\|'verified'\|'installing'\|'error'` → nothing / indeterminate `ProgressBar` / info `Banner` / info `Banner` + **Descargar** / determinate `ProgressBar` / info `Banner` + **Instalar** (`installDisabled`/`Reason`) / indeterminate `ProgressBar` / critical `Banner` + retry. Optional `lastCheckLabel`/`Value`. No channel (rule 20) |
| `about` | `SettingsAboutSection` | version row, Repetir introducción, `links[]` (`external: true` draws the external-link glyph + `externalHint`), optional `technicalSummary` Snippet, optional folded Avanzado with `detailedLogging`, its `detailedLoggingUntilLabel` and `onDetailedLoggingChange` |
| `riskZone` | `SettingsRiskZoneSection` | "restablecer ThrottleWatch" (opens an internal confirm `Dialog`, `resetStatus` drives the same in-progress/error rendering as `privacy`'s delete action) |
| `footer` | `Snippet?` | optional extra content after "Zona de riesgo" (e.g. a build id) |

`SettingsScreen` reflows at its own container width via a CSS
`@container` query keyed to the same 700px breakpoint as
`tokens.BREAKPOINTS.compact` (stacking each `OptionRow`'s label above
its control) — it does not read `window.innerWidth` itself, unlike the
optional `createWidthTracker()` helper the app shell uses for its own
layout decisions (section 4). Host it inside a scrolling container;
it does not scroll itself.

### SessionCard
| prop | type | required | notes |
|---|---|---|---|
| `status` | `SessionStatus` (`'active'\|'completed'\|'cancelled'\|'incomplete'\|'imported'`) | yes | drives the pulsing dot (`active`) vs. `StatusChip` (`completed`/`imported` + `classification`) vs. muted tag (other) |
| `typeLabel` | `string` | yes | e.g. "Diagnóstico guiado" |
| `dateLabel` | `string` | yes | |
| `durationLabel` | `string` | no | |
| `statusLabel` | `string` | no | shown for the muted-tag statuses |
| `classification` | `Classification` | no | only meaningful with `status='completed'`/`'imported'` |
| `classificationLabel` | `string` | no | |
| `importedLabel` | `string` | no | shown next to `status='imported'` |
| `selected` | `boolean` | no | |
| `openLabel` | `string` | yes | |
| `onOpen` | `() => void` | yes | |
| `exportLabel` | `string` | no | omit to hide the export icon button |
| `onExport` | `() => void` | no | |
| `exportDisabled` | `boolean` | no | |
| `exportDisabledReason` | `string` | no | |
| `deleteLabel` | `string` | no | omit to hide the delete icon button entirely |
| `onRequestDelete` | `() => void` | no | fires immediately on click — this card has no confirmation of its own; `SessionsScreen` owns the one shared confirm `Dialog` |
| `isReference` / `referenceLabel` | `boolean` / `string` | no | "Referencia" tag on a guided session marked for before/after comparisons (only guided sessions can be references) |
| `referenceActionLabel` / `onToggleReference` | `string` / `() => void` | no | star toggle, rendered only for `completed` |

### SessionsScreen
| prop | type | required | notes |
|---|---|---|---|
| `status` | `SessionsListStatus` (`'loading'\|'error'\|'loaded'`) | yes | |
| `sessions` | `SessionsScreenSession[]` | yes | `SessionCard`'s props plus a required `id` |
| `title` / `importLabel` / `onImport` / `importDisabled` | `string` / `string` / `() => void` / `boolean` | no | optional header with the "Importar…" action (opens the native picker in the host) |
| `onDeleteSession` | `(id: string) => void` | yes | fires only after the shared confirm `Dialog` is accepted |
| `loadingLabel` | `string` | yes | |
| `emptyTitle` / `emptyDescription` / `emptyIcon` / `emptyAction` | see `EmptyState` | no | shown when `sessions.length === 0` and `status='loaded'` |
| `errorTitle` / `errorDescription` | `string` | yes / no | shown via `Banner tone="critical"` |
| `retryLabel` / `onRetry` | `string` / `() => void` | yes | the error `Banner`'s action |
| `deleteDialogTitle` / `deleteDialogDescription` / `deleteDialogCancelLabel` / `deleteDialogConfirmLabel` | `string` | yes | the one shared confirm `Dialog`'s copy |

Reflows at its own container width (`@container tw-sessions`, 599px) —
`SessionCard`'s result row wraps to a new line below that width.

### CoreCell
| prop | type | required | notes |
|---|---|---|---|
| `core` | `CoreReading` (`{ id, index, group?, temperatureLabel?, clockLabel?, tone: Tone, unavailable?, throttling? }`) | yes | |
| `metricMode` | `CoreMetricMode` (`'temperature'\|'clock'`) | yes | which label to show |
| `tooltipLabel` | `(core: CoreReading) => string` | yes | full temp/carga/reloj/throttling readout, rendered inside `Tooltip` |
| `selected` | `boolean` | no | |
| `onSelect` | `(id: string) => void` | no | |

`unavailable` renders a diagonal hatch background and `'—'` instead of
a value; `throttling` adds a 2px `status-thermal` bottom-edge bar. Both
are visual states, never a color swap alone, matching the "reduced
confidence reads as a pattern, not just a dimmer color" rule used
elsewhere (`AnalysisChart`'s dashed segments).

### CpuTopologyMap
| prop | type | required | notes |
|---|---|---|---|
| `groups` | `CoreGroup[]` (`{ kind: CoreGroupKind \| 'ungrouped', label?, cores: CoreReading[] }`) | yes | a homogeneous CPU is one group with `kind: 'ungrouped'` and no `label`; a hybrid one is 2–3 labeled groups. No special-cased branching for "homogeneous vs. hybrid" — it just renders whatever groups it's given. |
| `metricMode` | `CoreMetricMode` | yes | |
| `metricModeLabel` / `temperatureModeLabel` / `clockModeLabel` | `string` | yes | the toggle `SegmentedControl`'s labels |
| `tooltipLabel` | `(core: CoreReading) => string` | yes | passed straight through to each `CoreCell` |
| `onMetricModeChange` | `(mode: CoreMetricMode) => void` | no | |
| `selectedCoreId` | `string` | no | |
| `onSelectCore` | `(id: string) => void` | no | |
| `coverageTone` / `coverageTitle` / `coverageDescription` | `Tone` / `string` / `string` | no | an optional `Banner` for partial per-core coverage — omit `coverageTitle` to hide it entirely |

### CpuAdvancedTable
| prop | type | required | notes |
|---|---|---|---|
| `rows` | `CoreTableRow[]` | yes | `{ id, index, groupLabel?, temperatureLabel, temperatureValue?, clockLabel, clockValue?, loadLabel, loadValue?, throttlingLabel, unavailable? }` — the host supplies both the display label AND the raw sortable value; this component sorts numbers, it never parses "78°" back into 78 |
| `columnLabels` | `CoreTableColumnLabels` | yes | `{ index, group, temperature, clock, load, throttling }` |
| `filterLabel` / `filterPlaceholder` | `string` | yes / no | |
| `emptyFilterMessage` | `string` | yes | shown when the filter matches nothing |
| `rowHeight` | `number` | no (`36`) | drives the virtualization math — don't change it without re-checking scroll behavior |
| `maxHeight` | `number` | no (`320`) | viewport height before scrolling kicks in |

Filter (substring match on core index/group) and sort (click a header)
are generic list operations this component owns itself — like
`Select`'s own open/closed state, not the kind of domain judgment this
system otherwise keeps out of its components. Virtualized with a
hand-rolled windowed render (rule 13) — a core with `unavailable` still
gets a row, it's never dropped by the filter unless its own
index/group text doesn't match.

### CpuScreen
Composes `CpuTopologyMap` + `CpuAdvancedTable`. Owns `metricMode` as
local view-only state (same category as `SessionsScreen`'s
dialog-open state) — the temperature/clock toggle is a display
choice, not domain data the host needs to track.
| prop | type | required | notes |
|---|---|---|---|
| `title` | `string` | yes | |
| `topologyLabel` / `tableLabel` | `string` | yes | section headings |
| `groups` | `CoreGroup[]` | yes | passed straight to `CpuTopologyMap` |
| `metricModeLabel` / `temperatureModeLabel` / `clockModeLabel` | `string` | yes | |
| `tooltipLabel` | `(core: CoreReading) => string` | yes | |
| `selectedCoreId` / `onSelectCore` | `string` / `(id: string) => void` | no | |
| `coverageTone` / `coverageTitle` / `coverageDescription` | see `CpuTopologyMap` | no | |
| `tableRows` | `CoreTableRow[]` | yes | passed straight to `CpuAdvancedTable` |
| `tableColumnLabels` / `tableFilterLabel` / `tableFilterPlaceholder` / `tableEmptyFilterMessage` | see `CpuAdvancedTable` | yes/yes/no/yes | |

### GuidedDiagnosticScreen
Single component driven entirely by `phase` — see rule 12. Renders its
own fixed 5-step stepper (Preflight/Calentamiento/Carga estable/
Recuperación/Resultado) and a battery-state `Banner` when
`batteryState !== 'ok'`.
| prop | type | required | notes |
|---|---|---|---|
| `phase` | `DiagnosticPhase` (13 values: `'intro'\|'preflight'\|'ready'\|'rest'\|'warming'\|'steadyLoad'\|'recovery'\|'cancelling'\|'cancelled'\|'result'\|'safetyStop'\|'sensorLost'\|'error'`) | yes | `rest` is the optional stabilisation wait (`restTitle`/`restDescription`/`skipRestLabel`/`onSkipRest`) |
| `whatWillHappenTitle` / `whatWillHappen` / `introDisclaimer` | `string` / `{ label, value }[]` / `string` | no | the structured "Qué va a pasar" list on `intro` (HU-04) |
| `useAsReferenceLabel` / `onUseAsReference` / `referenceNote` | `string` / `() => void` / `string` | no | on `result` |
| `batteryState` | `BatteryState` (`'ok'\|'warning'\|'blocked'`) | yes | |
| `preflightChecks` | `PreflightCheck[]` | no | shown during `'preflight'` |
| `reading` | `DiagnosticLiveReading` | no | shown during `'warming'`/`'steadyLoad'`/`'recovery'` as a `StatWidget` pair |
| `result` | `DiagnosticResult` | no | shown during `'result'` via `StatusChip` |
| `onStart` / `onStop` / `onCancel` / `onRetry` / `onViewReport` / … | `() => void` | no | see `GuidedDiagnosticScreen.svelte`'s own `Props` interface for the exact phase-specific action callbacks and their labels |
| `stepLabels` | `[string, string, string, string, string, string]` | yes | the 6 stepper captions (Comprobación/Reposo/Calentamiento/Carga sostenida/Recuperación/Resultado), in that fixed order — no copy lives in this package (rule 2) |
| `stepperLabel` | `string` | yes | `aria-label` for the `role="list"` stepper |
| `preflightFailedTitle` | `string` | yes | `Banner` title shown when one or more `preflightChecks` has `status: 'failed'` |
| `preflightCheckingLabel` | `string` | yes | `ProgressBar` label shown while `phase === 'preflight'` |
| `temperatureStatLabel` / `limitStatLabel` / `headroomStatLabel` | `string` | yes/yes/no | the `StatWidget` labels in the live-reading row (`reading`; `headroomLabel`/`remainingLabel` are optional fields of it) |

`showStop` (Detener ahora) is true for preflight/ready/rest/warming/
steadyLoad/recovery/cancelling (disabled while cancelling);
`showClosingActions` is true for cancelled/safetyStop/sensorLost/
error/result — both are internal rendering rules, not props, so don't
try to override them from the host.

### AnalysisChart
Hand-rolled SVG synced multi-track chart (rule 13). One `<svg>`
contains every track stacked by Y-offset, so a single pointer-move
handler draws one shared cursor across all of them.
| prop | type | required | notes |
|---|---|---|---|
| `tracks` | `AnalysisTrack[]` (`{ kind: AnalysisTrackKind, label, unit?, tone: Tone, points: AnalysisPoint[], min, max }`) | yes | `AnalysisPoint = { t: number; value: number \| null; reducedQuality?: boolean }` — see rule 14 for `null` |
| `events` | `AnalysisEvent[]` (`{ id, kind: 'thermal'\|'electrical'\|'mixed', startT, endT, label }`) | no | `'mixed'` renders a diagonal two-color `<pattern>`, never a blended third color |
| `selectedEventId` | `string` | no | |
| `onSelectEvent` | `(id: string \| undefined) => void` | no | fires on click of an event band (toggles) |
| `timeLabel` | `(t: number) => string` | yes | formats the hover readout's time axis label |
| `legendLabel` | `string` | yes | accessible name for the plot + legend group |
| `trackHeight` | `number` | no (`64`) | per-track pixel height inside the shared `<svg>` |
| `onRangeSelect` | `(range: [number, number] \| null) => void` | no | fires from the drag-to-select brush strip below the plot; `null` means the selection was cleared |
| `valueLabel` | `(point: AnalysisPoint \| undefined, track: AnalysisTrack) => string` | yes | formats a single value for the hover readout AND the hidden range table — owns 100% of the wording, including "sin dato" and any reduced-quality suffix; the component never builds this string itself (rule 2) |
| `cursorLabel` | `string` | yes | `aria-label` for the `.brush` control (see keyboard model below) — replaces what used to be `legendLabel` misapplied to this element |
| `rangeSummaryLabel` | `(range: [number, number]) => string` | yes | one sentence summarizing a `[startIndex, endIndex]` range — used as both the visible one-line summary under the chart and the `aria-valuetext` while a range is armed/committed |
| `resetRangeLabel` | `string` | yes | visible text of the "clear range" button, shown only when `committedRange` is non-null |

Clicking a legend swatch shows/hides that track locally (view-only
state this component owns itself, per rule 12).

**Keyboard model for `.brush` (`role="slider"`, single moving value —
see rule 17 below for why there's only one slider, not two):**
- `ArrowLeft` / `ArrowRight` — move the cursor one sample at a time.
- `Home` / `End` — jump the cursor to the first / last sample.
- `Enter` / `Space` — arm a range anchor at the cursor (first press),
  then commit the range from anchor to current cursor (second press).
  Committing fires `onRangeSelect([lo, hi])`; committing a zero-width
  range (arrow keys never moved) fires `onRangeSelect(null)` instead,
  matching "no selection", not a 1-sample range.
- `Escape` — cancels an armed-but-not-yet-committed selection without
  touching any previously committed range.
- `aria-valuenow` always reflects the cursor position (0…`points.length
  - 1`); `aria-valuetext` is the cursor's `timeLabel` normally, and
  switches to `rangeSummaryLabel(...)` while a selection is armed or
  committed — so the semantics stay a real single-value slider
  throughout (rule 17), the *content* of `aria-valuetext` is what
  communicates "you're now selecting a range."
- A visible one-line `rangeSummaryLabel(...)` sentence and a visually-
  hidden real `<table>` (per-track start/end `valueLabel`s via
  `<caption>`/`<th scope="col">`/`<th scope="row">`) render right below
  `.brush` whenever a range is armed or committed — the textual/tabular
  alternative to the visual chart, for screen-reader and "find in page"
  use alike.

#### AnalysisChart: SVG vs. ECharts

**Decision: `AnalysisChart`'s hand-rolled SVG is the definitive
production implementation, not a visual reference meant to move to
ECharts later.** It's covered by the same "zero runtime dependencies"
constitution as `CpuAdvancedTable`'s virtualization (rule 13), and this
is the one place that constraint gets a real performance budget
attached, verified rather than asserted, so a consumer can build
against it with confidence:

- **Node-count strategy.** A track is drawn as ONE `<path>` per
  contiguous run of same-quality, non-null samples — never one SVG
  node per data point. A `null` sample (a real gap, rule 14) or a
  `reducedQuality` transition starts a new segment/path; everything
  else is coordinates inside a single path's `d` attribute, which is
  cheap for the DOM regardless of point count. Practical effect:
  rendering 4 tracks × 6,000 points each (24,000 samples) with 100
  event bands produced 688 SVG nodes total (348 of them `<path>`s) —
  not 24,000.
- **Verified performance envelope** (measured with Playwright driving
  the real component in this harness, Chromium, `dist/` build — not
  asserted): time from navigation to first paint of `svg.plot`, and
  per-interaction latency for `ArrowRight` (moves the keyboard cursor)
  and a pointer sweep across the plot, at increasing scale, 4 tracks
  throughout:

  | points/track | events | SVG nodes | time to render | per-keypress | per-pointer-move |
  |---|---|---|---|---|---|
  | 300 | 20 | 92 | 167 ms | 2.65 ms | 16.3 ms |
  | 600 | 20 | 108 | 149 ms | 2.40 ms | 16.3 ms |
  | 1,500 | 40 | 224 | 154 ms | 2.38 ms | 16.4 ms |
  | 3,000 | 60 | 376 | 172 ms | 2.47 ms | 16.7 ms |
  | 6,000 | 100 | 688 | 178 ms | 3.08 ms | 16.4 ms |
  | 12,000 | 100 | 1,056 | 269 ms | 4.63 ms | — |
  | 20,000 | 100 | 1,544 | 281 ms | 2.97 ms | — |

  Even at 20,000 points/track (80,000 samples across 4 tracks) plus 100
  event bands, first render stays under 300 ms and keyboard-cursor
  latency stays single-digit-ms — headroom well beyond what a
  reasonable session view needs (see below).
- **Recommended max points per track: ~2,000–3,000 for a single
  session view.** This isn't a hard technical ceiling (the measurements
  above show the component comfortably handles an order of magnitude
  more) — it's a *readability* ceiling: past a couple thousand points,
  a fixed-width chart has more samples than horizontal pixels, so
  individual points stop being visually meaningful anyway. Treat the
  performance headroom as margin for real devices being slower than
  this harness's Chromium, not as license to skip aggregation.
- **Recommended max total points (all tracks combined): ~10,000–12,000**
  for the same readability reason, scaled by however many tracks are
  visible at once (currently up to 4).
- **Yes — data should arrive pre-aggregated/reduced from Rust for long
  sessions.** `AnalysisChart` has no aggregation logic of its own (it
  draws exactly the points it's given, rule 12's "no business logic"
  boundary applies here too) and shouldn't grow any — downsample on the
  Rust side (e.g. min/max/avg bucketing, or LTTB) before a session
  exceeds the per-track guidance above, the same way you'd already
  decide a fixed pixel budget for any chart library. This keeps
  `reducedQuality` meaningful as "this specific sample was a lower-
  fidelity reading," not overloaded to also mean "this is a downsampled
  bucket."
- **Long-session behavior:** for a session past the recommended point
  budget, aggregate before handing data to this component rather than
  raising the prop's expectations — the component's contract stays
  "however many points you pass, it draws them all, in one path per
  gap-free run," regardless of session length.
- **If a future need ever exceeds this envelope** (a track genuinely
  needs hundreds of thousands of points live, or many more than 4
  tracks at once), that's a reason to introduce a canvas-based or
  virtualized-viewport renderer *behind this same component's props* —
  not a reason to adopt ECharts, since nothing here depends on ECharts'
  actual feature set (its own scale/zoom/tooltip system would still
  need wrapping to match this component's ARIA-correct keyboard model
  in §2 above, rule 17). If that ever happens, the wrapper keeps
  `AnalysisChart`'s existing prop surface so no consuming screen has to
  change.

### AnalysisScreen
| prop | type | required | notes |
|---|---|---|---|
| `status` | `AnalysisStatus` (`'noHistory'\|'loading'\|'ready'`) | yes | |
| `loadingLabel` / `noHistoryTitle` / `noHistoryDescription` / `noHistoryIcon` | see `ProgressBar`/`EmptyState` | yes/yes/no/no | |
| `dataNoticeTone` / `dataNoticeTitle` / `dataNoticeDescription` | `'warning'\|'critical'` / `string` / `string` | no | one optional `Banner` slot for "datos obsoletos"/"datos parciales" — omit `dataNoticeTitle` to hide it |
| `tracks` / `events` / `selectedEventId` / `onSelectEvent` / `timeLabel` / `legendLabel` / `onRangeSelect` | see `AnalysisChart` | | passed straight through |
| `valueLabel` / `cursorLabel` / `rangeSummaryLabel` / `resetRangeLabel` | see `AnalysisChart` | yes | passed straight through to the internal `AnalysisChart` |
| `selectedEventEvidence` / `rangeEvidence` | `AnalysisEvidence` (`{ title, description?, items?: { label, value }[] }`) | no | priority: `selectedEventEvidence` > `rangeEvidence` > `idleHint` — the host computes both by looking up `events`/the range, `AnalysisScreen` never does that lookup itself |
| `idleHint` | `string` | yes | shown when neither evidence prop is set |
| `evidenceTitle` | `string` | yes | the evidence panel's heading |

Reflows at its own container width (`@container tw-analysis`, 699px) —
the evidence panel moves below the chart instead of beside it.

### ReportScreen
Narrative sections — see rule 16 for the "omit, don't pad" contract.
| prop | type | required | notes |
|---|---|---|---|
| `status` | `'loading'\|'ready'` | yes | |
| `loadingLabel` | `string` | yes | |
| `classification` / `classificationLabel` | `Classification` / `string` | yes | rendered via `StatusChip` |
| `headline` | `string` | yes | "resultado en una frase" |
| `provisional` / `provisionalNoticeTitle` / `provisionalNoticeDescription` | `boolean` / `string` / `string` | no | info `Banner` while the session is still running (FR-067): the report is live and not yet frozen |
| `reevaluated` | `{ title, classification, classificationLabel, headline, note? }` | no | imported session re-evaluated with the current ruleset (FR-073), shown in a dashed block next to — never instead of — the original |
| `sessionIncomplete` / `incompleteNoticeTitle` / `incompleteNoticeDescription` | `boolean` / `string` / `string` | no | independent `Banner`, stacks above `reducedConfidence`'s |
| `reducedConfidence` / `reducedConfidenceNoticeTitle` / `reducedConfidenceNoticeDescription` | `boolean` / `string` / `string` | no | independent `Banner` — both notices can show at once, a session can be incomplete AND on reduced confidence |
| `observedTitle` / `observedText` | `string` | yes | "qué se observó" |
| `causalChain` | `CausalNode[]` | no | maps straight to `CausalRail` — only pass it with a real 2–4 node chain (see `CausalRail`'s own rule); leave undefined otherwise |
| `impactTitle` | `string` | yes | |
| `impactValue` | `string` | no | see rule 15 — absent means show the unavailable state instead |
| `impactUnavailableTitle` / `impactUnavailableReason` | `string` | no | shown when `impactValue` is absent |
| `evidenceTitle` / `evidence` | `string` / `string[]` | yes / no | plain bullet list; section omitted when `evidence` is empty |
| `alternativeCausesTitle` / `alternativeCauses` | `string` / `string[]` | yes / no | |
| `cannotConcludeTitle` / `cannotConclude` | `string` / `string[]` | yes / no | |
| `recommendationsTitle` / `recommendations` | `string` / `string[]` | yes / no | rendered as a numbered list |
| `methodTitle` | `string` | yes | |
| `methodDescription` | `string` | no | how the impact was obtained: power-headroom inputs (PL1, measured power) or the guided test; shown when a method applies |
| `methodMissingText` | `string` | no | shown instead when no method applies (e.g. power limit unknown) — this section always renders (never omitted), it just has two possible bodies |
| `comparisonTitle` / `comparisonMetrics` | `string` / `ReportComparisonMetric[]` (`{ label, beforeValue, afterValue }`) | yes / no | before/after table; omitted when `comparisonMetrics` is empty |
| `comparisonBeforeLabel` / `comparisonAfterLabel` | `string` | yes (whenever `comparisonMetrics` is non-empty) | the table's own two column headers — no copy lives in this package (rule 2) |
| `exportActions` | `Snippet` | no | compose with `Button` (e.g. "Exportar PDF", "Copiar resumen") |

## 3. Shared vocabulary

```ts
type Classification =
  | 'normal' | 'hot_unproven' | 'thermal_probable' | 'thermal_confirmed'
  | 'power_limited' | 'mixed_limit' | 'indeterminate';

type Tone = 'accent' | 'thermal' | 'warm' | 'normal' | 'power' | 'unknown';
```

`CLASSIFICATION_META[classification]` (in `lib/classification.ts`) is
the only place classification → color/icon is decided. If a screen
needs an 8th classification, add it there first, not inline in a
component call.

## 4. Responsive wiring (optional helper)

```ts
import { createWidthTracker } from '$lib/design-system/lib/responsive.svelte';
const win = createWidthTracker(); // win.tier: 'compact' | 'medium' | 'expanded'
```

Use `win.tier` to decide: `BottomBar` vs. sidebar, `NavigationItem`
density, whether to mount `CausalRail` (`'expanded'` only). You can
also implement this yourself against `BREAKPOINTS` (`{ compact: 700,
expanded: 980 }`) from `tokens/tokens.ts` if the app already has its
own resize-tracking store — don't run two separate width trackers.

## 5. Verification already done

All eleven batches were type-checked (`svelte-check`, 0 errors/0
warnings) and production-built (`vite build`) against Svelte 5.57 +
TypeScript — see section 6 for the harness that reproduces this
yourself. `examples/AhoraScreen.example.svelte` was screenshotted at
380px / 860px / 1400px in both themes to confirm the three-tier reflow
and the `StatusHero` container-query breakpoint actually fire, and
again after switching it to `NavIcon` for all 6 destinations and again
after `StatusHero`'s internal refactor onto `ProgressBar` (no visual
regression). `examples/SettingsSection.example.svelte` was
screenshotted in both themes, including the `Dialog` opened via
`Button`, to confirm the native `<dialog>` backdrop, the deepened
button fills, and the disabled-row dimming on `OptionRow` all render
correctly. `examples/OnboardingIllustrations.example.svelte` was
screenshotted in both themes to confirm the 5 illustrations theme
correctly through the real components (not just the raw SVG this was
prototyped in). The app icon was rendered at 1024px then downsampled
to 16/32/48px to check it still reads as a partial ring at taskbar
size, and packed into a 7-size `.ico` that was re-opened with Pillow to
confirm every size landed. `examples/MorePrimitives.example.svelte` was
screenshotted in both themes, plus two interaction-state captures:
`Tooltip` open (hovering a core tile) and `Select`'s popup open — to
confirm both actually render their open state correctly, not just
their closed/default appearance. `examples/OnboardingFlow.example.svelte`
was screenshotted across slide 1 (dark+light), mid-flow (slide 2, to
confirm "Atrás" appears and the progress dots move), and slide 5 in
all three detection states (`detecting`/`complete`/`partial`), partial
coverage stacked with the advanced-access-unavailable banner, and the
resumed-session note — dark for all of these plus one light-theme spot
check on the `complete` state. `examples/SettingsScreen.example.svelte`
(the full `SettingsScreen` component, batch 6) was screenshotted at
the three width tiers (420px compact, 860px medium, 1100px expanded)
in dark theme, plus expanded in light theme, to confirm the
`@container` reflow (rows stack under 700px) and both themes render
correctly; and in five additional interaction states: sensor coverage
`'complete'` (the local checkmark, not `StatusIcon`), the update flow's
`'available'` and `'downloading'` states (reached by actually clicking
"Buscar actualizaciones" then "Instalar y reiniciar", not just passed
as a static prop), the update flow's `'error'` state (critical banner
+ retry), and both destructive-action confirmation dialogs open
(`Dialog` over "Eliminar todos mis datos" and over "Restablecer
ThrottleWatch"). If you change a component's markup or CSS, re-check
the matching scenario before considering the change done — a broken
reflow, an inaccessible disabled state, an icon that turns to mush
below 32px, a popup/tooltip that never actually opens in a screenshot,
an onboarding detection state that was never actually driven to that
state and screenshotted, or a `SettingsScreen` status that was only
ever passed as `'idle'`, is the most likely regression here.

`examples/SessionsScreen.example.svelte` (batch 7) was screenshotted
covering all 5 `SessionStatus` values in one seeded list plus the
forced-empty state, and the shared delete-confirmation `Dialog` open.
`examples/GuidedDiagnosticScreen.example.svelte` (batch 8) was
screenshotted across all 12 `DiagnosticPhase` values and all 3
`BatteryState` values via a `Select`-driven demo picker.
`examples/CpuScreen.example.svelte` (batch 9) was screenshotted for the
hybrid P/E/LP topology (dark+light), the homogeneous topology, the
64-core topology (to confirm `CpuAdvancedTable` actually virtualizes —
scrolling the table and re-screenshotting confirmed the visible row
range shifted from core 0 to core ~22, i.e. old rows were unmounted
and replaced, not just scrolled within a full DOM), a hovered core tile
(`Tooltip` open), the temperature/clock metric toggle, and the table's
filter+sort interaction. A narrow-width (420px) column-truncation bug
in `CpuAdvancedTable` was caught this way and fixed by adding a
horizontal-scroll container with a `min-width` on its rows instead of
letting columns shrink below legibility.
`examples/AnalysisScreen.example.svelte` (batch 10) was screenshotted
in both themes (idle state), hovering the chart (cursor + per-track
readout, including the real gap reading as "sin dato"), clicking a
thermal event band (evidence panel switches to `selectedEventEvidence`
plus a red band outline), drag-selecting a range on the brush strip
(evidence panel switches to `rangeEvidence`, plus a "Restablecer zoom"
action), the `noHistory` and `loading` states, the data-notice `Banner`
(partial data), hiding a track via its legend swatch, and a 420px
compact width (evidence panel moves below the chart). A dedicated
high-DPI element screenshot of just the `.plot` `<svg>` was taken to
confirm, pixel-for-pixel, that the temperature gap is a real broken
line segment (not interpolated), the load track's reduced-quality run
renders with a dashed stroke, and the `'mixed'` event band renders the
actual two-color diagonal stripe pattern (not a single blended color).
`examples/ReportScreen.example.svelte` (batch 11) was screenshotted for
the full narrative with a causal chain (dark+light), the loading state,
both the session-incomplete and reduced-confidence banners stacked
together with the impact-unavailable state (no method), the
`normal` classification with the causal chain hidden (confirming it's
omitted, not rendered empty), the `mixed_limit` classification's
neutral chip, an export-action button click, and a 420px compact width
(confirming the header wraps the export buttons below the chip and the
comparison table's columns narrow via its own `@container` rule).

One real bug this process caught: a literal `<style>`/`<script>`
substring inside a `<script lang="ts">` doc comment (e.g. writing
"see the `<style>` block" in prose) makes the Svelte 5 compiler report
`element_unclosed` on that file — it's scanning raw script text for
those two tag openers even inside a comment. If you need to refer to
the style block or the script tag in a doc comment, spell it as "the
style block" / "the script tag", never `<style>`/`<script>` literally.

`OnboardingFlow.svelte` seeds its local `currentStep` state from the
`initialStep` prop once, on mount (`$state(untrack(() => ...))`) — this
is the standard Svelte 5 "prop as an initial value, then owned
locally" pattern, not a bug. Without `untrack`, the compiler warns
`state_referenced_locally` on that line; it's a false positive for
this exact pattern, and `untrack` is the documented way to say "read
this once, don't track it" instead of silencing the warning some other
way.

`Tooltip.svelte`'s anchor `<span>` carries two
`<!-- svelte-ignore -->` comments (`a11y_no_noninteractive_element_interactions`,
`a11y_no_static_element_interactions`) — this is intentional, not a
suppressed real bug. The span is a hover/focus positioning zone, not
itself an interactive widget (the actual button/tile is the
consumer's, rendered via `children`), so it can't correctly take an
ARIA role, and the linter has no rule shape for "non-interactive
wrapper that only forwards hover/focus timing." Leave both comments in
place if you touch this file.

**i18n + AnalysisChart-accessibility pass (post-batch-11):** every
remaining hardcoded visible or accessible string was removed from
`Banner` (`dismissLabel`), `TitleBar` (`minimizeLabel`/`maximizeLabel`/
`restoreLabel`/`closeLabel`), `GuidedDiagnosticScreen` (`stepLabels`/
`stepperLabel`/`preflightFailedTitle`/`preflightCheckingLabel`/
`temperatureStatLabel`/`limitStatLabel`), `ReportScreen`
(`comparisonBeforeLabel`/`comparisonAfterLabel`), and `AnalysisChart`
(`valueLabel`/`cursorLabel`/`rangeSummaryLabel`/`resetRangeLabel`,
replacing the `'sin dato'`/`'calidad reducida'` string-building and the
`'Restablecer zoom'` button text that used to live inside the
component). Every consuming example, `AnalysisScreen`, and this file's
own §2/§0.5 were updated to match, then re-verified with `npm run check`
(0 errors/0 warnings, 139 files) and `npm run build`. Regression
screenshots of `guided`, `report`, `ahora`, and `more` confirmed no
visual change from the prop renames.

`AnalysisChart`'s keyboard/ARIA model was verified programmatically
(not just visually) with Playwright driving the built harness: tabbing
to `.brush` and reading its live `role`/`aria-label`/`aria-valuemin`/
`aria-valuemax`/`aria-valuenow`/`aria-valuetext`/`tabindex`; 10×
`ArrowRight` incrementing `aria-valuenow`; `End`/`Home` jumping to the
extremes; `Enter` arming a range, 20× `ArrowRight` extending it (mid-
selection `aria-valuetext` switches to the `rangeSummaryLabel`
sentence), a second `Enter` committing it (fires `onRangeSelect`,
renders the visible `.range-summary` line and the hidden
`table.sr-only` with correct per-track start/end values via
`valueLabel`); the reset button clearing the range (`table.sr-only`
disappears); and `Escape` canceling an armed-but-uncommitted selection
without touching a prior committed range. Every assertion passed on
the first implementation.

The "AnalysisChart: SVG vs. ECharts" decision (§2, under
`AnalysisChart`) was verified, not just asserted: a temporary
Playwright-driven stress harness (not shipped — created and deleted
within `harness/src/` for this one check) rendered `AnalysisChart`
directly with synthetic 1–4 track datasets from 300 up to 20,000
points/track plus 20–100 event bands, confirming (a) SVG node count
tracks the number of gap/quality-transition boundaries, not the number
of points — a track renders as one `<path>` per contiguous run, so
20,000 points/track × 4 tracks + 100 events produced 1,544 SVG nodes,
not 80,000+ — and (b) first render stayed under 300ms and keyboard
(`ArrowRight`) interaction latency stayed single-digit-millisecond
across the entire range tested. The point-budget guidance in that
section (~2,000–3,000/track, ~10,000–12,000 total) is a readability
ceiling, not this measured technical one — pass pre-aggregated data
from Rust once a session exceeds it.

## 6. Reproducible verification harness

`harness/` is a small, self-contained npm subproject shipped inside
this package specifically so the checks in section 5 aren't just a
claim — you can run them yourself against the exact tool versions
pinned in `harness/package.json` (`svelte` 5.57.0 exactly, plus pinned
`vite`/`svelte-check`/`typescript`). It is not part of the design
system itself — don't import from `harness/` — and don't copy it into
a host app; copy everything else one level up (`components/`,
`icons/`, `illustrations/`, `lib/`, `tokens/`, `brand/`) instead.

```sh
cd throttlewatch-design-system/harness
npm install
npm run check    # svelte-check + tsc — expect 0 errors, 0 warnings
npm run build    # vite build — expect a clean production build
npm run preview  # serves the build; open the printed localhost URL
```

`npm run preview` serves a real, browsable nav (`harness/src/App.svelte`)
over every `examples/*.example.svelte` file plus a dark/light theme
toggle button — not just a URL you have to already know the shape of.
It also still reads a `?view=<id>` query param on load and keeps it in
sync as you click around, so a screenshot script can deep-link
directly to one example (e.g. `?view=settings`) the same way every
screenshot referenced in section 5 was taken (see
`harness/src/App.svelte`'s `views` array for the exact list and which
example file each one loads). To regenerate any of the screenshots
this file describes, drive that URL with Playwright (or any automated
browser) against `http://localhost:<preview-port>/?view=<id>`, toggle
`.theme-toggle` for the light-theme captures, and resize the viewport
for the compact/medium/expanded shots — that is exactly how they were
produced. The nav's current ids: `ahora`, `settings-primitives`,
`settings`, `sessions`, `guided`, `cpu`, `analysis`, `report`,
`onboarding`, `onboarding-flow`, `more`.

`harness/tsconfig.app.json`'s `include` reaches up into the real
package folders (`../components/**/*.svelte`, `../examples/**/*.svelte`,
etc.) rather than copying anything, so `npm run check` here type-checks
the actual shipped source, not a snapshot of it — if you edit a
component in place, re-run `npm run check` from `harness/` and it
picks up the change immediately.

One genuine bug this harness caught that the older, less strict
verification setup used earlier in this project had missed:
`examples/OnboardingFlow.example.svelte` originally built its `steps`
array with `$derived([...])` (the expression form). TypeScript's
control-flow narrowing saw the `let detectionStatus: DetectionStatus =
$state('detecting')` declaration and, finding no reassignment between
that line and the `detectionStatus === 'complete'` comparisons a few
lines later in the *same* lexical scope (the reassignments live inside
separate `onRetry`/`onRequestAdvancedAccess` closures, which don't
count for narrowing), narrowed `detectionStatus` to the literal
`'detecting'` and flagged every comparison as `TS2367` ("this condition
will always return false"). The fix — confirmed with a minimal
standalone `tsc --noEmit --strict` repro outside Svelte — is wrapping
the expression in a function scope: `$derived.by(() => [...])` instead
of `$derived([...])`. This resets TypeScript's narrowing and is
otherwise behaviorally identical. If you see `TS2367` on a comparison
against a `$state`-declared union-typed variable that *is* reassigned,
just inside a callback rather than inline, reach for `$derived.by`
before assuming the comparison itself is wrong.
