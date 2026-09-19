<script lang="ts">
  /**
   * A small (2–5 option) mutually-exclusive choice, styled as a native
   * connected segmented control rather than a dropdown — matches how
   * Ajustes actually describes its own choices: Idioma ("Usar idioma
   * del sistema" / "Español" / "English"), Apariencia ("Sistema" /
   * "Claro" / "Oscuro"), Monitorización ("Bajo consumo" / "Normal" /
   * "Diagnóstico"), retención de datos (4 options). Reach for a
   * dropdown/select only when you have more options than comfortably
   * fit as segments — this system doesn't ship one because nothing in
   * the spec currently needs it.
   *
   * `value` is bindable:
   *
   *   <SegmentedControl bind:value={theme} label="Apariencia"
   *     options={[{value:'system',label:'Sistema'},{value:'light',label:'Claro'},{value:'dark',label:'Oscuro'}]} />
   */
  interface SegmentOption {
    value: string;
    label: string;
  }

  interface Props {
    options: SegmentOption[];
    value?: string;
    label: string;
    disabled?: boolean;
    onchange?: (value: string) => void;
  }

  let { options, value = $bindable(options[0]?.value ?? ''), label, disabled = false, onchange }: Props = $props();

  function select(v: string) {
    if (disabled) return;
    value = v;
    onchange?.(v);
  }
</script>

<div class="tw-segmented" class:disabled role="radiogroup" aria-label={label}>
  {#each options as opt (opt.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === opt.value}
      class="segment label"
      class:selected={value === opt.value}
      disabled={disabled}
      onclick={() => select(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .tw-segmented {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background-color: var(--glass-bg-subtle);
    background-image: var(--glass-sheen);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-edge);
    border-radius: var(--radius-md);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-segmented.disabled {
    opacity: 0.4;
    pointer-events: none;
  }
  .segment {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }
  .segment {
    transition:
      background-color var(--motion-base) var(--motion-ease-out),
      color var(--motion-base) linear,
      transform var(--motion-fast) var(--motion-spring);
  }
  .segment:active {
    transform: scale(0.96);
  }
  .segment.selected {
    background: var(--glass-bg-strong);
    color: var(--text-primary);
    box-shadow: var(--glass-shadow);
    animation: tw-bump var(--motion-base) var(--motion-spring);
  }
  .segment:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
</style>
