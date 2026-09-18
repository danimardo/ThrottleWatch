<script lang="ts">
  /**
   * Translates evidence into a readable sequence ("sustained load ->
   * temperature at the limit -> effective clock dropped -> estimated
   * lower performance"). The consumer supplies 2 to 4 nodes — never
   * pad it with an invented step just to fill the row.
   *
   * THIS COMPONENT HAS NO EMPTY STATE. If the sequence is not backed
   * by sufficient evidence, do not render <CausalRail> at all — its
   * "empty state" is to not exist in the layout. Likewise, the app
   * shell should not mount it below a 980px window width tier (see
   * README breakpoint table / lib/responsive.svelte.ts): it needs
   * horizontal room to read as a sequence and does not have a
   * scroll or stacked fallback.
   *
   * The first node (the root cause, usually "Carga") is almost always
   * tone="accent" — it is context, not yet an alert. Later nodes use
   * whichever tone the diagnosis engine identified: all the same tone
   * if the chain is purely thermal, mixing "thermal" and "power" if the
   * engine flagged a mixed cause.
   *
   * Connectors are always neutral (surface-sunken), never colored —
   * color lives in the nodes so this reads as a data diagram, not a
   * marketing infographic.
   */
  import type { Snippet } from 'svelte';
  import { TONE_TOKENS, type Tone } from '../tokens/tokens';

  export interface CausalNode {
    id: string;
    label: string;
    value: string;
    tone: Tone;
    icon: Snippet;
  }

  interface Props {
    nodes: CausalNode[];
  }

  let { nodes }: Props = $props();

  $effect(() => {
    if (import.meta.env?.DEV && (nodes.length < 2 || nodes.length > 4)) {
      console.warn(
        `[ThrottleWatch/CausalRail] received ${nodes.length} nodes — this component expects 2 to 4. ` +
          'Do not pad the sequence with an invented step; if you have fewer than 2, do not render this component at all.'
      );
    }
  });
</script>

<div class="rail">
  {#each nodes as node, i (node.id)}
    <div class="node" style:--tw-i={i}>
      <div
        class="tile"
        style:background={`color-mix(in srgb, var(--${TONE_TOKENS[node.tone]}) 12%, transparent)`}
        style:color={`var(--${TONE_TOKENS[node.tone]})`}
      >
        {@render node.icon()}
      </div>
      <span class="caption" style:color="var(--text-tertiary)">{node.label}</span>
      <span class="body-strong">{node.value}</span>
    </div>
    {#if i < nodes.length - 1}
      <div class="connector" style:--tw-i={i}></div>
    {/if}
  {/each}
</div>

<style>
  @keyframes tw-draw {
    from {
      transform: scaleX(0);
      opacity: 0;
    }
    to {
      transform: scaleX(1);
      opacity: 1;
    }
  }
  .rail {
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    border-radius: var(--radius-lg);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
    display: flex;
    align-items: flex-start;
  }
  .node {
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    animation-delay: calc(var(--tw-i, 0) * var(--motion-stagger) * 2);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    text-align: center;
  }
  .tile {
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .tile :global(svg) {
    width: 15px;
    height: 15px;
  }
  .connector {
    transform-origin: left center;
    animation: tw-draw var(--motion-slow) var(--motion-ease-out) both;
    animation-delay: calc((var(--tw-i, 0) * 2 + 1) * var(--motion-stagger));
    flex: 0 0 auto;
    width: 38px;
    height: 2px;
    background: var(--surface-sunken);
    margin: 16px 2px 0;
    border-radius: 2px;
  }
</style>
