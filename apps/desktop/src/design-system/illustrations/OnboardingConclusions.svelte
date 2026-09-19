<script lang="ts">
  /**
   * Onboarding slide 3 — "Qué puede concluir" (diferencia entre
   * temperatura alta, limitación térmica, límite de potencia y datos
   * insuficientes).
   *
   * blocks: a fan of four partial rings, each on its own muted track —
   *   normal (small, ~20% filled: barely a limit), warm (medium,
   *   ~45%), thermal (larger, ~75%: the most confirmed-looking one),
   *   plus a fourth ring with NO fill at all — a dashed outline
   *   instead of a colored arc, standing in for "indeterminate": the
   *   distinction is drawn structurally (dashed vs. solid), never as
   *   a fifth invented color, matching lib/classification.ts's own
   *   rule that indeterminate is a different KIND of thing from a
   *   confirmed-but-small reading, not just another point on the same
   *   scale
   * scale: canvas 280x220, radii 30/44/34/22
   */
  interface Props {
    width?: number | string;
    height?: number | string;
  }
  let { width = '100%', height = '100%' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 280 220" role="img" aria-hidden="true">
  <circle cx="66" cy="130" r="30" fill="none" stroke="var(--surface-sunken)" stroke-width="10" />
  <circle
    cx="66" cy="130" r="30" fill="none" stroke="var(--status-normal)" stroke-width="10"
    stroke-linecap="round" stroke-dasharray="188.5" stroke-dashoffset="151"
    transform="rotate(-90 66 130)"
  />
  <circle cx="140" cy="85" r="44" fill="none" stroke="var(--surface-sunken)" stroke-width="13" />
  <circle
    cx="140" cy="85" r="44" fill="none" stroke="var(--status-warm)" stroke-width="13"
    stroke-linecap="round" stroke-dasharray="276.5" stroke-dashoffset="152"
    transform="rotate(-90 140 85)"
  />
  <circle cx="214" cy="130" r="34" fill="none" stroke="var(--surface-sunken)" stroke-width="11" />
  <circle
    cx="214" cy="130" r="34" fill="none" stroke="var(--status-thermal)" stroke-width="11"
    stroke-linecap="round" stroke-dasharray="213.6" stroke-dashoffset="53"
    transform="rotate(-90 214 130)"
  />
  <circle
    cx="140" cy="180" r="22" fill="none" stroke="var(--status-unknown)" stroke-width="6"
    stroke-dasharray="3.5 6.5" stroke-linecap="round"
  />
</svg>
