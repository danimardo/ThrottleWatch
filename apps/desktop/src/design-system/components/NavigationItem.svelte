<script lang="ts">
  /**
   * One row of the primary sidebar (Ahora, Análisis, CPU, Sesiones,
   * Diagnóstico guiado, Ajustes). Renders in one of two densities —
   * the density itself is decided by the app shell from the window
   * width tier (see lib/responsive.svelte.ts), not by this component:
   * a NavigationItem does not know how wide the window is, it is only
   * told whether to show its label.
   *
   *   'labeled'   -> icon + label            (width tier >= 980px)
   *   'icon-only' -> icon only, centered     (700–979px)
   *
   * Below 700px the sidebar disappears entirely in favor of BottomBar
   * — that is a decision the app shell makes about which component to
   * render, not a third density of this one.
   *
   * Active state is background+color in accent-blue, never a status
   * color: which item is active is a navigation fact, not a diagnostic
   * one.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    icon: Snippet;
    label: string;
    active?: boolean;
    density?: 'labeled' | 'icon-only';
    onclick?: () => void;
  }

  let { icon, label, active = false, density = 'labeled', onclick }: Props = $props();
</script>

<button
  type="button"
  class="navitem label"
  class:active
  class:icon-only={density === 'icon-only'}
  aria-current={active ? 'page' : undefined}
  aria-label={density === 'icon-only' ? label : undefined}
  {onclick}
>
  <span class="icon">{@render icon()}</span>
  {#if density === 'labeled'}
    <span class="text">{label}</span>
  {/if}
</button>

<style>
  .navitem {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    white-space: nowrap;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    transition:
      background-color var(--motion-base) var(--motion-ease-out),
      color var(--motion-base) linear,
      transform var(--motion-fast) var(--motion-spring);
  }
  .navitem:active {
    transform: scale(0.97);
  }
  .navitem.icon-only {
    justify-content: center;
    padding: 8px;
  }
  .navitem .icon {
    width: 16px;
    height: 16px;
    flex: 0 0 auto;
    color: var(--text-tertiary);
    display: flex;
  }
  .navitem .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .navitem:hover {
    background: var(--surface-raised);
  }
  .navitem.active {
    background: color-mix(in srgb, var(--accent-blue) 14%, var(--glass-bg-subtle));
    color: var(--accent-blue);
    box-shadow: var(--glass-edge);
    animation: tw-pop var(--motion-base) var(--motion-spring);
  }
  .navitem.active .icon {
    color: var(--accent-blue);
  }
  .navitem:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
