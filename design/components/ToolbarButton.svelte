<script lang="ts">
  /**
   * A secondary action aligned to the right of a header row (e.g. "Ver
   * cobertura"). This is a toolbar action, never a filled/pill
   * call-to-action button: no background at rest, accent-blue
   * text/icon, a faint background only on hover/focus.
   *
   * `compact` is provided by the app shell (window width < 700px) —
   * below that width the label is dropped and the button becomes
   * icon-only, but it never loses its accessible name.
   *
   * Do not use this for a destructive or irreversible action (delete
   * data, reset, stop a running test) — those need their own
   * confirmation treatment, not a discreet toolbar button.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    icon: Snippet;
    label: string;
    compact?: boolean;
    onclick?: () => void;
    disabled?: boolean;
  }

  let { icon, label, compact = false, onclick, disabled = false }: Props = $props();
</script>

<button
  type="button"
  class="tb-btn label"
  aria-label={label}
  {onclick}
  {disabled}
>
  <span class="icon">{@render icon()}</span>
  {#if !compact}
    <span class="text">{label}</span>
  {/if}
</button>

<style>
  .tb-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--accent-blue);
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    border: none;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tb-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-blue) 8%, transparent);
  }
  .tb-btn:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .tb-btn:disabled {
    color: var(--text-tertiary);
    cursor: default;
  }
  .icon {
    width: 15px;
    height: 15px;
    display: flex;
    flex: 0 0 auto;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
