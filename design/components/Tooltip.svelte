<script lang="ts">
  /**
   * The accessibility spec is explicit and specific here: "Tooltips
   * accesibles por teclado y que no desaparecen al mover el puntero
   * hacia ellos." A native `title` attribute fails BOTH halves of that
   * sentence — it never triggers from keyboard focus, and it vanishes
   * the instant the pointer leaves the trigger, before it can reach
   * the tooltip itself. This component exists specifically to make
   * that requirement satisfiable at all: the CPU topology map's
   * per-core tooltip (temperature/carga/reloj efectivo/throttling) and
   * Análisis's chart tooltips both need it.
   *
   * Shows on pointer hover AND on keyboard focus, and stays open while
   * the pointer is over the tooltip panel itself (a short grace period
   * on leave/blur, cancelled if the pointer/focus re-enters either the
   * trigger or the panel). Escape closes it and returns focus to the
   * trigger.
   *
   * You own the actual interactive element (a button, a tile) — this
   * component does not render one for you, because it can't know if
   * your trigger should be a button, a link, or something else. It
   * hands you the tooltip's id back through its `children` snippet so
   * you can wire `aria-describedby` yourself:
   *
   *   <Tooltip label="98°C · Carga 92% · 4.1GHz · throttling 12s">
   *     {#snippet children({ describedBy })}
   *       <button class="core-tile" aria-describedby={describedBy}>P2</button>
   *     {/snippet}
   *   </Tooltip>
   *
   * For anything richer than one line, pass `body` instead of `label`
   * (same override pattern as Dialog's `body`/`description`).
   */
  import type { Snippet } from 'svelte';

  interface ChildrenArgs {
    describedBy: string | undefined;
  }

  interface Props {
    label?: string;
    body?: Snippet;
    placement?: 'top' | 'bottom' | 'left' | 'right';
    children: Snippet<[ChildrenArgs]>;
  }

  let { label, body, placement = 'top', children }: Props = $props();

  const id = `tw-tooltip-${Math.random().toString(36).slice(2, 9)}`;
  let open = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  function show() {
    clearTimeout(hideTimer);
    open = true;
  }
  function scheduleHide() {
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (open = false), 140);
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      open = false;
    }
  }
</script>

<!--
  This wrapper is a hover/focus zone, not itself an interactive widget —
  the actual interactive element (button, tile, etc.) is the consumer's,
  rendered via `children`. That's why it stays role-less and the linter
  rule for "non-interactive element with listeners" is silenced here.
-->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="tw-tooltip-anchor" onpointerenter={show} onpointerleave={scheduleHide} onfocusin={show} onfocusout={scheduleHide} onkeydown={onKeydown}>
  {@render children({ describedBy: open ? id : undefined })}
  {#if open}
    <span class="tw-tooltip {placement}" role="tooltip" {id} onpointerenter={show} onpointerleave={scheduleHide}>
      {#if body}
        {@render body()}
      {:else if label}
        {label}
      {/if}
    </span>
  {/if}
</span>

<style>
  .tw-tooltip-anchor {
    position: relative;
    display: inline-flex;
  }
  .tw-tooltip {
    position: absolute;
    z-index: 30;
    max-width: 240px;
    width: max-content;
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    color: var(--text-primary);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    animation: tw-pop var(--motion-fast) var(--motion-ease-out) both;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    font-size: 11px;
    line-height: 14px;
    font-weight: 650;
    letter-spacing: 0.01em;
    pointer-events: auto;
  }
  .tw-tooltip.top {
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
  }
  .tw-tooltip.bottom {
    top: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
  }
  .tw-tooltip.left {
    right: calc(100% + 6px);
    top: 50%;
    transform: translateY(-50%);
  }
  .tw-tooltip.right {
    left: calc(100% + 6px);
    top: 50%;
    transform: translateY(-50%);
  }
</style>
