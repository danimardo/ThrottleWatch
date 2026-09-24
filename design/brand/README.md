# ThrottleWatch — brand assets

Three things live here, all derived from the same "anillo parcial"
concept approved for the app icon — the thermal ring gauge, the
product's one signature shape (the same motif as the Cover component
in the separate ThrottleWatch visual Design System artifact — not a
file in this Svelte package), drawn chunkier than the in-app UI ring
so it survives being shrunk to 16px.

```
app-icon/
  icon.png          1024x1024 master (source of truth — re-export everything else from this if you ever change it)
  icon.ico          Windows multi-resolution icon (16/24/32/48/64/128/256 baked in)
  32x32.png
  128x128.png
  128x128@2x.png    256x256
  source.html       editable SVG source (open in a browser, or re-render with a headless browser)

tray/               five system-tray states (T068, FR-059), two variants each
  generate.mjs      source of truth — regenerates every .svg/.png below (see "Tray icons")
  <state>-<variant>.svg          editable vector, one per state/variant
  <state>-<variant>-<size>.png   16/20/24/32/48px renders of the same svg
```

## Wiring into Tauri

Point `tauri.conf.json`'s bundle icon list at this folder (copy it to
`src-tauri/icons/` first, or reference it wherever your project layout
expects icons):

```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ]
  }
}
```

`icon.ico` is what Windows actually uses for the taskbar, the window's
own title-bar icon (the small mark to the left of "ThrottleWatch" in
`components/TitleBar.svelte` is a separate, smaller SVG mark you supply
via its `icon` snippet — it does not have to be this same file, though
reusing this ring keeps the window icon and the taskbar icon
consistent) and the installer/shortcut. The PNGs are what Tauri's
bundler resamples for anything else it needs at build time.

If you later add other platforms or want the complete official set
(including sizes this folder doesn't ship), regenerate everything from
`icon.png` with the Tauri CLI instead of hand-editing PNGs:

```
cargo tauri icon path/to/icon.png
```

## Tray icons (T068, FR-059)

Five states — `normal`, `warning`, `critical`, `unknown`, `disconnected` — each in a `dark`
variant (for a dark taskbar: light-toned neutral strokes) and a `light` variant (for a light
taskbar: dark-toned neutral strokes). The three semantic states (`normal`/`warning`/`critical`)
use the exact same hex colors as `apps/desktop/src-tauri/src/tray.rs::TrayIconState::color` in
**both** variants — green/amber/red already read against a light or a dark background, so only
`unknown` and `disconnected`, which have no real "brand color" of their own, switch their neutral
tone per variant.

Same "anillo parcial" motif as `app-icon/` (a track ring at the same stroke-to-radius ratio,
~0.36), but a **full** ring rather than a partial arc: unlike the app icon's thermal gauge, a
tray state is not a live percentage, so implying one here would be misleading. Every state adds a
distinct **shape** on top of its color — never color alone (design system rule 5, and the same
"a pattern, not just a dimmer color" principle `AnalysisChart`'s reduced-quality dashes and
`CoreCell`'s hatch already use):

| State | Ring | Extra mark |
|---|---|---|
| `normal` | solid, green | none |
| `warning` | solid, amber | small center dot |
| `critical` | solid, red | large center dot |
| `unknown` | dashed, neutral | — (the dash pattern itself is the mark) |
| `disconnected` | solid, neutral | diagonal slash (the universal "no signal" glyph) |

Regenerate after touching `generate.mjs` (it needs `@resvg/resvg-js`, installed ad hoc — this
tool is not a repository dependency, same as `app-icon`'s "re-render with a headless browser"):

```sh
cd design/brand/tray
npm install @resvg/resvg-js --no-save
node generate.mjs .
```

### Wired into the tray

`tray.rs::refresh` used to paint a small colored dot over the app's default window icon
(`overlay_dot`) as a placeholder; it now embeds the 32px PNGs (`include_bytes!`) per
`TrayIconState`, copied into `apps/desktop/src-tauri/icons/tray/` the same way `app-icon/icon.ico`
is copied into `icons/` — this folder stays the editable source, that one is the build input. It
picks the `dark`/`light` variant from `UISettings.GetColorValue`'s background luminance (the same
WinRT surface T131's `appearance.rs` already reads for `UISettings.AdvancedEffectsEnabled` — see
`tray_variant()`), defaulting to `dark` when that read fails, since most Windows installs default
to a dark taskbar.

## Why this shape specifically

A taskbar/shortcut icon is seen mostly at 16-32px, where a design that
reads fine inside the app (like StatusHero's slender 11px-stroke ring)
turns into a gray smear. This icon uses the same ring but with a much
thicker stroke (110px on a 1024px canvas — roughly 5x thicker in
proportion to its radius than the in-app version) specifically so the
shape still reads as "a partial ring" rather than "a blurry circle"
once Windows shrinks it. The colors are hardcoded to the dark-theme
token values (`--bg` / `--surface-sunken` / `--status-thermal`) rather
than left as CSS variables, because an OS-level icon file can't
respond to the app's own light/dark toggle the way an in-app SVG can —
it always ships in the dark-native identity, which is also this
system's default theme.
