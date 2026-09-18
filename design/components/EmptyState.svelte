<script lang="ts">
  /**
   * A centered placeholder for "nothing here yet" — Análisis before
   * enough history exists, CPU's topology map before the first reading
   * arrives, a filtered Sesiones list with no matches. No card, no
   * border by default: it sits directly on the surface it occupies
   * rather than drawing a box around the absence of content, matching
   * the system's own "elevation is a hairline, used sparingly" instinct
   * — an empty section doesn't need elevating at all.
   *
   * `icon` is consumer-supplied (a NavIcon, a StatusIcon, or any custom
   * glyph) rather than one of a fixed set — unlike StatusIcon/NavIcon,
   * an empty state's illustration has no closed vocabulary to protect.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    icon?: Snippet;
    title: string;
    description?: string;
    action?: Snippet;
  }

  let { icon, title, description, action }: Props = $props();
</script>

<div class="tw-empty-state">
  {#if icon}
    <div class="icon">{@render icon()}</div>
  {/if}
  <span class="value-md" style:color="var(--text-primary)">{title}</span>
  {#if description}
    <span class="body" style:color="var(--text-secondary)">{description}</span>
  {/if}
  {#if action}
    <div class="action">{@render action()}</div>
  {/if}
</div>

<style>
  .tw-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-8) var(--space-6);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-empty-state .icon {
    color: var(--text-tertiary);
    width: 40px;
    height: 40px;
    margin-bottom: var(--space-2);
  }
  .tw-empty-state .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .tw-empty-state .body {
    max-width: 320px;
  }
  .tw-empty-state .action {
    margin-top: var(--space-3);
  }
</style>
