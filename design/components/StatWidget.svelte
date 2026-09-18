<script lang="ts">
  /**
   * A tile for one individual signal (temperature, load, effective
   * clock, power). Group several inside a container using:
   *
   *   grid-template-columns: repeat(auto-fit, minmax(150px, 1fr))
   *
   * 1–4 columns emerge from the available width with no dedicated
   * breakpoint needed for this component.
   *
   * `tone` is chosen by what this widget is showing RIGHT NOW, not a
   * fixed per-metric assignment — a temperature widget in normal range
   * uses tone="normal", not tone="thermal". Load/Power default to
   * "accent" (informative, usually not the cause) unless an active
   * limit makes them the story.
   *
   * A missing sensor does not hide the widget: pass value="No
   * disponible" (or the EN string) with tone="unknown" and explain why
   * in `footnote` — keep the layout's space, don't collapse it.
   */
  import type { Snippet } from 'svelte';
  import { TONE_TOKENS, type Tone } from '../tokens/tokens';

  interface Props {
    icon: Snippet;
    tone: Tone;
    label: string;
    value: string;
    unit?: string;
    footnote: string;
    /** Position in the grid, for the staggered entrance (`--tw-i`). */
    enterIndex?: number;
  }

  let { icon, tone, label, value, unit, footnote, enterIndex }: Props = $props();
  let token = $derived(TONE_TOKENS[tone]);

  // Value-change bump: a brief scale on the number whenever `value`
  // changes (not on first render). Decorative; the text itself is
  // always the real value (README "Motion").
  let bump = $state(false);
  let prevValue: string | undefined;
  $effect(() => {
    const v = value;
    if (prevValue !== undefined && prevValue !== v) {
      bump = true;
      const t = setTimeout(() => (bump = false), 450);
      prevValue = v;
      return () => clearTimeout(t);
    }
    prevValue = v;
  });
</script>

<div class="stat" class:bump class:tw-enter={enterIndex !== undefined} style:--tw-i={enterIndex}>
  <div class="th">
    <div class="tile" style:background={`color-mix(in srgb, var(--${token}) 12%, transparent)`} style:color={`var(--${token})`}>
      {@render icon()}
    </div>
    <span class="label" style:color="var(--text-secondary)">{label}</span>
  </div>
  <div class="value-md value-row">
    {value}{#if unit}<span class="body" style:color="var(--text-secondary)">{unit}</span>{/if}
  </div>
  <div class="caption" style:color="var(--text-tertiary)">{footnote}</div>
</div>

<style>
  .stat {
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    transition:
      transform var(--motion-base) var(--motion-spring),
      box-shadow var(--motion-base) var(--motion-ease-out);
  }
  .stat:hover {
    transform: translateY(-2px);
    box-shadow: var(--glass-shadow-strong);
  }
  .value-row {
    transform-origin: left center;
  }
  .stat.bump .value-row {
    animation: tw-bump var(--motion-slow) var(--motion-spring);
  }
  .stat {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .th {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .tile {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }
  .tile :global(svg) {
    width: 11px;
    height: 11px;
  }
</style>
