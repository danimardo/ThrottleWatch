<script lang="ts">
  /**
   * Onboarding slide 5 — "Este equipo" (comienza automáticamente la
   * detección pasiva, muestra CPU, estado del colector y cobertura
   * disponible).
   *
   * blocks: the CPU die outline (a bigger sibling of icons/NavIcon's
   *   "cpu" glyph) with a small accent-blue ring badge overlapping its
   *   bottom-right corner, read as "actively reading this chip right
   *   now" — the badge is the same partial-ring motif as every other
   *   slide, in accent-blue rather than status-thermal because at
   *   this point nothing has been diagnosed yet; this is detection in
   *   progress, an accent fact, not a status one
   * scale: canvas 280x220, die 120x120, badge ring radius 24
   */
  interface Props {
    width?: number | string;
    height?: number | string;
  }
  let { width = '100%', height = '100%' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 280 220" role="img" aria-hidden="true">
  <rect x="70" y="40" width="120" height="120" rx="18" fill="none" stroke="var(--text-tertiary)" stroke-width="3" />
  <rect x="95" y="65" width="42" height="42" rx="7" fill="none" stroke="var(--text-tertiary)" stroke-width="3" />
  <rect x="147" y="65" width="20" height="20" rx="4" fill="none" stroke="var(--text-tertiary)" stroke-width="3" />
  <rect x="147" y="93" width="20" height="20" rx="4" fill="none" stroke="var(--text-tertiary)" stroke-width="3" />
  <rect x="95" y="117" width="72" height="20" rx="4" fill="none" stroke="var(--text-tertiary)" stroke-width="3" />
  <circle cx="196" cy="150" r="30" fill="var(--bg)" />
  <circle cx="196" cy="150" r="24" fill="none" stroke="var(--surface-sunken)" stroke-width="9" />
  <circle
    cx="196" cy="150" r="24" fill="none" stroke="var(--accent-blue)" stroke-width="9"
    stroke-linecap="round" stroke-dasharray="150.8" stroke-dashoffset="60"
    transform="rotate(-90 196 150)"
  />
</svg>
