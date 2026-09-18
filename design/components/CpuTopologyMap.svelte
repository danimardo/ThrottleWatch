<script lang="ts">
  /**
   * The core grid: one or more groups of `CoreCell`, a shared
   * temperature/clock toggle, and an optional coverage banner. Renders
   * a homogeneous topology as a single ungrouped grid (pass one group
   * with `kind: 'ungrouped'`) and a hybrid P/E/LP topology as several
   * labeled groups, stacked — never as one undifferentiated grid,
   * which would hide the P/E/LP distinction the spec asks for.
   *
   * Presentational: this never reads real sensors. `tooltipLabel` is a
   * formatting callback (same pattern as `OnboardingFlow`'s
   * `progressLabel`) — the host already has the numbers and knows how
   * to phrase "78° · 62% carga · 4.1 GHz · throttling 12s"; this
   * component just calls it once per core, per render.
   *
   * A large core count (many-cores) needs no special prop — the grid
   * is `repeat(auto-fill, minmax(...))`, so it wraps naturally at any
   * width and any core count.
   */
  import CoreCell, { type CoreReading, type CoreGroupKind, type CoreMetricMode } from './CoreCell.svelte';
  import SegmentedControl from './SegmentedControl.svelte';
  import Banner from './Banner.svelte';

  export interface CoreGroup {
    kind: CoreGroupKind | 'ungrouped';
    label?: string;
    cores: CoreReading[];
  }

  interface Props {
    groups: CoreGroup[];
    metricMode: CoreMetricMode;
    metricModeLabel: string;
    temperatureModeLabel: string;
    clockModeLabel: string;
    tooltipLabel: (core: CoreReading) => string;
    onMetricModeChange: (mode: CoreMetricMode) => void;
    selectedCoreId?: string;
    onSelectCore?: (id: string) => void;
    coverageTone?: 'info' | 'warning';
    coverageTitle?: string;
    coverageDescription?: string;
  }

  let {
    groups,
    metricMode,
    metricModeLabel,
    temperatureModeLabel,
    clockModeLabel,
    tooltipLabel,
    onMetricModeChange,
    selectedCoreId,
    onSelectCore,
    coverageTone,
    coverageTitle,
    coverageDescription
  }: Props = $props();
</script>

<div class="tw-cpu-topology">
  <div class="toolbar">
    <SegmentedControl
      label={metricModeLabel}
      value={metricMode}
      onchange={(v) => onMetricModeChange(v as CoreMetricMode)}
      options={[
        { value: 'temperature', label: temperatureModeLabel },
        { value: 'clock', label: clockModeLabel }
      ]}
    />
  </div>

  {#if coverageTitle}
    <Banner tone={coverageTone ?? 'info'} title={coverageTitle} description={coverageDescription} />
  {/if}

  {#each groups as group (group.kind + (group.label ?? ''))}
    <div class="group">
      {#if group.label}
        <span class="caption group-label" style:color="var(--text-tertiary)">{group.label}</span>
      {/if}
      <div class="grid">
        {#each group.cores as core (core.id)}
          <CoreCell
            reading={core}
            {metricMode}
            tooltipLabel={tooltipLabel(core)}
            selected={core.id === selectedCoreId}
            onSelect={onSelectCore ? () => onSelectCore(core.id) : undefined}
          />
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .tw-cpu-topology {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .toolbar {
    display: flex;
    justify-content: flex-end;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    padding: var(--space-3);
  }
  .group-label {
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(46px, 1fr));
    gap: 6px;
  }
</style>
