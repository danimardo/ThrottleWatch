<script lang="ts">
  /**
   * A persistent in-page notice — "no se pudo leer el sensor X",
   * "actualización disponible", "modo de bajo consumo activo: algunas
   * lecturas se omiten" — the kind of thing that sits at the top of a
   * screen or section until the condition it describes changes or the
   * person dismisses it. Not a toast (it doesn't auto-expire) and not
   * a Dialog (it doesn't block the screen).
   *
   * Deliberately does NOT reuse StatusIcon. StatusIcon's six glyphs are
   * the closed set for `diagnostic_report.classification` (see
   * lib/classification.ts) — a banner's tone is a different semantic
   * domain (transient app/system notices, not a thermal/power
   * diagnosis) and reusing the same glyphs would blur that line the
   * first time a banner and a classification tag appeared on screen
   * together. This component draws its own three small local shapes.
   *
   * `tone="critical"` still isn't the same thing as a `thermal_confirmed`
   * StatusChip — it's for banner-worthy failures like "no se pudo
   * conectar con el sensor", not for restating a diagnosis.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    tone?: 'info' | 'warning' | 'critical';
    title: string;
    description?: string;
    action?: Snippet;
    onDismiss?: () => void;
    /** Accessible name for the dismiss (×) button — required whenever `onDismiss` is set. */
    dismissLabel?: string;
  }

  let { tone = 'info', title, description, action, onDismiss, dismissLabel }: Props = $props();

  const TONE_TOKEN: Record<'info' | 'warning' | 'critical', string> = {
    info: 'accent-blue',
    warning: 'status-warm',
    critical: 'status-thermal'
  };
</script>

<div class="tw-banner" style:--tw-banner-color={`var(--${TONE_TOKEN[tone]})`}>
  <span class="glyph" aria-hidden="true">
    {#if tone === 'info'}
      <svg viewBox="0 0 20 20" width="16" height="16">
        <circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.6" />
        <circle cx="10" cy="6.2" r="1" fill="currentColor" />
        <path d="M10 9.5 L10 14.3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    {:else if tone === 'warning'}
      <svg viewBox="0 0 20 20" width="16" height="16">
        <path d="M10 3 L18 16.5 L2 16.5 Z" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
        <path d="M10 8.3 L10 12" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        <circle cx="10" cy="14.3" r="1" fill="currentColor" />
      </svg>
    {:else}
      <svg viewBox="0 0 20 20" width="16" height="16">
        <circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.6" />
        <path d="M7.3 7.3 L12.7 12.7 M12.7 7.3 L7.3 12.7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    {/if}
  </span>

  <div class="body-col">
    <span class="body-strong" style:color="var(--text-primary)">{title}</span>
    {#if description}
      <span class="body" style:color="var(--text-secondary)">{description}</span>
    {/if}
    {#if action}
      <div class="action">{@render action()}</div>
    {/if}
  </div>

  {#if onDismiss}
    <button type="button" class="dismiss" aria-label={dismissLabel ?? ''} onclick={onDismiss}>
      <svg viewBox="0 0 16 16" width="12" height="12">
        <path d="M3.5 3.5 L12.5 12.5 M12.5 3.5 L3.5 12.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    </button>
  {/if}
</div>

<style>
  .tw-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    background-color: color-mix(in srgb, var(--tw-banner-color) 12%, var(--glass-bg));
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    border-left: 3px solid var(--tw-banner-color);
    border-radius: var(--radius-md);
    box-shadow: var(--glass-shadow);
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .glyph {
    flex: 0 0 auto;
    display: flex;
    color: var(--tw-banner-color);
    margin-top: 1px;
  }
  .body-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .action {
    margin-top: var(--space-2);
  }
  .dismiss {
    flex: 0 0 auto;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
  }
  .dismiss:hover {
    background: var(--surface-sunken);
    color: var(--text-primary);
  }
  .dismiss:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
</style>
