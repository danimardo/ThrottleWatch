<script lang="ts">
  /**
   * One core in the CPU topology map. Shows either its temperature or
   * its effective clock (`metricMode`, shared across the whole map so
   * every cell reads the same metric at once), colored by `tone` (the
   * same accent/thermal/warm/normal/power/unknown vocabulary as
   * `StatWidget`/`ProgressBar` — never a color invented just for this
   * cell). `Tooltip` carries the full reading (temp/carga/reloj
   * efectivo/throttling) on hover AND keyboard focus.
   *
   * A core with no data is never hidden or skipped — pass
   * `unavailable`, which swaps the value for a placeholder and draws a
   * diagonal hatch instead of a flat tone fill, so "no reading" reads
   * as a distinct pattern, not just a dimmer color (same principle as
   * Análisis's reduced-quality segments using a stroke pattern, not
   * color alone).
   */
  import Tooltip from './Tooltip.svelte';
  import { TONE_TOKENS, type Tone } from '../tokens/tokens';

  export type CoreGroupKind = 'p' | 'e' | 'lp';
  export type CoreMetricMode = 'temperature' | 'clock';

  export interface CoreReading {
    id: string;
    index: number;
    group?: CoreGroupKind;
    temperatureLabel?: string;
    clockLabel?: string;
    tone: Tone;
    throttling?: boolean;
    unavailable?: boolean;
  }

  interface Props {
    reading: CoreReading;
    metricMode: CoreMetricMode;
    tooltipLabel: string;
    selected?: boolean;
    onSelect?: () => void;
  }

  let { reading, metricMode, tooltipLabel, selected = false, onSelect }: Props = $props();

  let token = $derived(TONE_TOKENS[reading.tone]);
  let displayValue = $derived(
    reading.unavailable ? '—' : metricMode === 'temperature' ? (reading.temperatureLabel ?? '—') : (reading.clockLabel ?? '—')
  );
</script>

<Tooltip label={tooltipLabel}>
  {#snippet children({ describedBy })}
    <button
      type="button"
      class="tw-core-cell"
      class:unavailable={reading.unavailable}
      class:throttling={reading.throttling}
      class:selected
      style:--core-color={`var(--${token})`}
      aria-describedby={describedBy}
      aria-label={tooltipLabel}
      aria-pressed={onSelect ? selected : undefined}
      onclick={onSelect}
    >
      <span class="index caption">{reading.index}</span>
      <span class="value">{displayValue}</span>
    </button>
  {/snippet}
</Tooltip>

<style>
  .tw-core-cell {
    width: 100%;
    aspect-ratio: 1;
    min-width: 42px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--core-color) 16%, var(--glass-bg-subtle));
    box-shadow: var(--glass-edge);
    color: var(--text-primary);
    transition:
      transform var(--motion-fast) var(--motion-spring),
      background-color var(--motion-slow) linear,
      box-shadow var(--motion-base) var(--motion-ease-out);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    position: relative;
  }
  .tw-core-cell:hover {
    transform: translateY(-1px) scale(1.03);
    box-shadow: var(--glass-shadow);
  }
  .tw-core-cell .index {
    color: var(--text-tertiary);
  }
  .tw-core-cell .value {
    font-size: 12px;
    font-weight: 700;
  }
  .tw-core-cell.throttling::after {
    content: '';
    position: absolute;
    left: 4px;
    right: 4px;
    bottom: 2px;
    height: 2px;
    border-radius: var(--radius-full);
    background: var(--status-thermal);
  }
  .tw-core-cell.selected {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .tw-core-cell:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .tw-core-cell.unavailable {
    background-image: repeating-linear-gradient(
      45deg,
      color-mix(in srgb, var(--text-tertiary) 16%, transparent),
      color-mix(in srgb, var(--text-tertiary) 16%, transparent) 3px,
      transparent 3px,
      transparent 7px
    );
    background-color: var(--surface-sunken);
    color: var(--text-tertiary);
  }
  .tw-core-cell.unavailable .value {
    color: var(--text-tertiary);
  }
</style>
