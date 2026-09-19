<script lang="ts">
  /**
   * Onboarding slide 2 — "Qué observa" (temperatura, carga, reloj
   * efectivo, potencia mediante cuatro visuales simples).
   *
   * blocks: four tinted tiles, one per signal, each a small glyph in
   *   its own status/accent color — literally the same tone mapping
   *   StatWidget uses (thermal/accent/warm/power), just illustrative
   *   size instead of UI size
   * arrangement: a loose, slightly-rotated cluster (±4-6°) rather than
   *   a rigid grid, so it reads as illustration rather than a second
   *   copy of the real StatWidget row
   * scale: canvas 280x220, tiles ~60-68px
   */
  interface Props {
    width?: number | string;
    height?: number | string;
  }
  let { width = '100%', height = '100%' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 280 220" role="img" aria-hidden="true">
  <g transform="translate(72,60) rotate(-6)">
    <rect x="-34" y="-34" width="68" height="68" rx="16" fill="color-mix(in srgb, var(--status-thermal) 16%, var(--bg))" />
    <path
      d="M0 -18c0 8-8 10-8 20a11 11 0 0 0 22 0c0-4-2-6-4-8 0 3-2 5-3 5-2 0-4-2-2-6C6 -10 3 -14 0 -18Z"
      fill="var(--status-thermal)"
      transform="translate(0,3) scale(0.85)"
    />
  </g>
  <g transform="translate(190,55) rotate(5)">
    <rect x="-32" y="-32" width="64" height="64" rx="15" fill="color-mix(in srgb, var(--accent-blue) 16%, var(--bg))" />
    <path d="M-16 12 L-6 -4 L2 6 L10 -10 L18 -2" fill="none" stroke="var(--accent-blue)" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" />
  </g>
  <g transform="translate(80,155) rotate(4)">
    <rect x="-30" y="-30" width="60" height="60" rx="14" fill="color-mix(in srgb, var(--status-warm) 16%, var(--bg))" />
    <circle cx="0" cy="0" r="15" fill="none" stroke="var(--status-warm)" stroke-width="4" />
    <path d="M0 -8v9l6 4" fill="none" stroke="var(--status-warm)" stroke-width="4" stroke-linecap="round" />
  </g>
  <g transform="translate(196,160) rotate(-5)">
    <rect x="-32" y="-32" width="64" height="64" rx="15" fill="color-mix(in srgb, var(--status-power) 16%, var(--bg))" />
    <path d="M4 -16 -12 6 h8 l-2 12 14 -20 h-8 Z" fill="var(--status-power)" />
  </g>
</svg>
