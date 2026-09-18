<script lang="ts">
  /**
   * The app's own destination glyphs — replaces the generic
   * placeholder icons the first "Ahora" example screen used inline.
   * Each shape is deliberately specific to what its screen actually
   * contains rather than a stock icon-font symbol:
   *
   * - "now"      a small, chunky version of the app's own thermal
   *              ring gauge (the same shape as the app icon and
   *              StatusHero) — "Ahora" IS the live reading, so its
   *              glyph is that reading's own shape, not a clock.
   * - "analysis" three synced sparkline tracks with a shared rhythm,
   *              because Análisis is literally a synced multi-track
   *              chart (temperature/clock/load), not a single trend
   *              arrow.
   * - "cpu"      a die with differently SIZED inner blocks, echoing
   *              the spec's own topology rule ("borde/forma distingue
   *              P, E y LP; no solo color") instead of a generic
   *              chip-with-pins.
   * - "sessions" uneven-length horizontal bars — a history of runs of
   *              different duration, not a generic bullet list.
   * - "guided"   the ring again, with a small play wedge — a guided
   *              diagnostic is a focused, started run, not a bolt.
   * - "settings" a plain gear. Kept deliberately generic: unlike the
   *              others this glyph is genuinely universal, and
   *              reinventing it would cost recognizability for no
   *              gain. Redrawn to this system's own stroke spec
   *              (1.8-2.2, round joins) rather than pasted from an
   *              icon font.
   *
   * Usage — pass as the icon snippet anywhere a Snippet is expected:
   *
   *   {#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
   *   <NavigationItem icon={nowIcon} label="Ahora" ... />
   *
   * Sized by its parent (width/height: 100%, like every other icon in
   * this system) and colored with currentColor, so it inherits
   * NavigationItem/BottomBar's active-state accent-blue for free.
   */
  export type NavIconKind = 'now' | 'analysis' | 'cpu' | 'sessions' | 'guided' | 'settings';

  interface Props {
    kind: NavIconKind;
  }

  let { kind }: Props = $props();
</script>

{#if kind === 'now'}
  <svg viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="12" r="8" stroke="currentColor" stroke-opacity="0.28" stroke-width="3.2" />
    <path d="M12 4 A8 8 0 0 1 19.3 15.2" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" />
  </svg>
{:else if kind === 'analysis'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M3 7 L7.5 4 L12 8 L16.5 4.5 L21 7" />
    <path d="M3 12.5 L8 10.5 L12.5 14 L17 11 L21 12.5" />
    <path d="M3 18 L8.5 16.5 L14 18.5 L21 16.5" />
  </svg>
{:else if kind === 'cpu'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="3" />
    <rect x="6.5" y="6.5" width="6" height="6" rx="1" />
    <rect x="14.5" y="6.5" width="3" height="3" rx="0.6" />
    <rect x="14.5" y="11" width="3" height="3" rx="0.6" />
    <rect x="6.5" y="14.5" width="10.5" height="3" rx="0.6" />
  </svg>
{:else if kind === 'sessions'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round">
    <path d="M4 6h9" />
    <path d="M4 12h14" />
    <path d="M4 18h6" />
  </svg>
{:else if kind === 'guided'}
  <svg viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity="0.28" stroke-width="1.8" />
    <path d="M12 3 A9 9 0 0 1 20.8 10.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
    <path d="M10 8.5v7l6-3.5-6-3.5Z" fill="currentColor" />
  </svg>
{:else if kind === 'settings'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="3.2" />
    <path d="M12 3.5v2.3M12 18.2v2.3M20.5 12h-2.3M5.8 12H3.5M17.7 6.3l-1.6 1.6M7.9 16.1l-1.6 1.6M17.7 17.7l-1.6-1.6M7.9 7.9 6.3 6.3" />
  </svg>
{/if}
