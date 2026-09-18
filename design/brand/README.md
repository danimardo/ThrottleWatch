# ThrottleWatch — brand assets

Two things live here, both derived from the same "anillo parcial"
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
