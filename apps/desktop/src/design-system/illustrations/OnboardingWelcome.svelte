<script lang="ts">
  /**
   * Onboarding slide 1 — "Bienvenida" ("Entiende si el calor está
   * limitando tu CPU"). Not Corporate-Memphis figurative art on
   * purpose: this system's whole visual language is instrument-calm,
   * so its onboarding stays in the same abstract/geometric vocabulary
   * as the Cover component (in the separate ThrottleWatch visual
   * Design System artifact) instead of switching to flat human mascots
   * for five slides and then back to hairlines and gauges for the
   * rest of the app.
   *
   * blocks: ground = bg · a single status-thermal ring, ~74% drawn
   * pattern: two thin accent-blue guide circles behind it — the same
   *   "instrument" cue Cover's arcs use, at low opacity
   * scale: canvas 280x220 (matches the other 4 onboarding slides so
   *   they sit in one carousel without per-slide sizing); ring radius
   *   58, stroke 18 — chunkier than StatusHero's in-app ring (130px,
   *   stroke 11) because this reads at illustration scale, not gauge
   *   scale
   */
  interface Props {
    width?: number | string;
    height?: number | string;
  }
  let { width = '100%', height = '100%' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 280 220" role="img" aria-hidden="true">
  <circle cx="140" cy="110" r="95" fill="none" stroke="var(--accent-blue)" stroke-opacity="0.14" stroke-width="1.5" />
  <circle cx="140" cy="110" r="78" fill="none" stroke="var(--accent-blue)" stroke-opacity="0.20" stroke-width="1.5" />
  <circle cx="140" cy="110" r="58" fill="none" stroke="var(--surface-sunken)" stroke-width="18" />
  <circle
    cx="140" cy="110" r="58" fill="none" stroke="var(--status-thermal)" stroke-width="18"
    stroke-linecap="round" stroke-dasharray="364.4" stroke-dashoffset="95"
    transform="rotate(-90 140 110)"
  />
</svg>
