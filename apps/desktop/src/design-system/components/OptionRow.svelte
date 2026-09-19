<script lang="ts">
  /**
   * One row of Ajustes: a label, an optional short description, and a
   * trailing control (Switch, SegmentedControl, Button, or a plain
   * value + chevron for a link-style row) — the layout every bullet
   * in the settings spec describes ("interruptor de...", "elige
   * entre...", each with a short description).
   *
   * The spec's own rule: "Los controles con dependencias no
   * desaparecen: se muestran deshabilitados y explican la condición."
   * Pass `disabled` + `disabledReason` together to satisfy that — this
   * row dims itself and swaps in the reason text, but it does not
   * reach into your `control` snippet to disable it for you. Set the
   * same `disabled` value on the control you render inside it:
   *
   *   <OptionRow label="Iniciar oculto en la bandeja"
   *     disabled={!trayMonitoringEnabled}
   *     disabledReason="Requiere activar la monitorización en bandeja.">
   *     {#snippet control()}
   *       <Switch bind:checked={startHidden} disabled={!trayMonitoringEnabled} label="Iniciar oculto en la bandeja" />
   *     {/snippet}
   *   </OptionRow>
   */
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    description?: string;
    disabled?: boolean;
    disabledReason?: string;
    control: Snippet;
  }

  let { label, description, disabled = false, disabledReason, control }: Props = $props();
</script>

<div class="row" class:disabled>
  <div class="text">
    <span class="body-strong" style:color="var(--text-primary)">{label}</span>
    {#if disabled && disabledReason}
      <span class="caption" style:color="var(--text-tertiary)">{disabledReason}</span>
    {:else if description}
      <span class="body" style:color="var(--text-secondary)">{description}</span>
    {/if}
  </div>
  <div class="control">
    {@render control()}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-5);
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--hairline);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .row:last-child {
    border-bottom: none;
  }
  .row.disabled {
    opacity: 0.55;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .control {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
  }
</style>
