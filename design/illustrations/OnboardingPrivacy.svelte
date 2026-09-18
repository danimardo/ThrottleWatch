<script lang="ts">
  /**
   * Onboarding slide 4 — "Privacidad y decisiones" (funcionamiento
   * local, ausencia de telemetría, historial de siete días).
   *
   * blocks: a dashed rounded-square perimeter (the boundary of "this
   *   device") with the thermal ring fully inside it and nothing
   *   crossing that boundary outward
   * pattern: the dashed line itself IS the pattern — deliberately not
   *   a padlock glyph. A padlock implies something being locked/
   *   guarded; what this slide actually says is simpler and calmer
   *   (nothing ever leaves), and a boundary that contains the ring
   *   with no lines escaping communicates that without introducing a
   *   new icon vocabulary this system doesn't otherwise use
   * scale: canvas 280x220, boundary 170x160 rounded 24, ring radius 42
   */
  interface Props {
    width?: number | string;
    height?: number | string;
  }
  let { width = '100%', height = '100%' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 280 220" role="img" aria-hidden="true">
  <rect
    x="55" y="30" width="170" height="160" rx="24" fill="none"
    stroke="var(--accent-blue)" stroke-opacity="0.35" stroke-width="2" stroke-dasharray="7 7"
  />
  <circle cx="140" cy="110" r="42" fill="none" stroke="var(--surface-sunken)" stroke-width="14" />
  <circle
    cx="140" cy="110" r="42" fill="none" stroke="var(--status-thermal)" stroke-width="14"
    stroke-linecap="round" stroke-dasharray="263.9" stroke-dashoffset="70"
    transform="rotate(-90 140 110)"
  />
</svg>
