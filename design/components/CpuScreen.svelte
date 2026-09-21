<script lang="ts">
  /**
   * The "CPU" screen: `CpuTopologyMap` above `CpuAdvancedTable`, both
   * driven from the same per-core data the host supplies. Owns one
   * piece of local, view-only state — the temperature/clock toggle —
   * the same category of state `SessionsScreen` owns for its delete
   * dialog: which way to *look* at the data, never a fact about it.
   */
  import CpuTopologyMap, { type CoreGroup } from './CpuTopologyMap.svelte';
  import CpuAdvancedTable, { type CoreTableRow, type CoreTableColumnLabels } from './CpuAdvancedTable.svelte';
  import type { CoreReading, CoreMetricMode } from './CoreCell.svelte';

  interface Props {
    title: string;
    topologyLabel: string;
    tableLabel: string;
    groups: CoreGroup[];
    metricModeLabel: string;
    temperatureModeLabel: string;
    clockModeLabel: string;
    tooltipLabel: (core: CoreReading) => string;
    selectedCoreId?: string;
    onSelectCore?: (id: string) => void;
    coverageTone?: 'info' | 'warning';
    coverageTitle?: string;
    coverageDescription?: string;
    tableRows: CoreTableRow[];
    tableColumnLabels: CoreTableColumnLabels;
    tableFilterLabel: string;
    tableFilterPlaceholder?: string;
    tableEmptyFilterMessage: string;
  }

  let {
    title,
    topologyLabel,
    tableLabel,
    groups,
    metricModeLabel,
    temperatureModeLabel,
    clockModeLabel,
    tooltipLabel,
    selectedCoreId,
    onSelectCore,
    coverageTone,
    coverageTitle,
    coverageDescription,
    tableRows,
    tableColumnLabels,
    tableFilterLabel,
    tableFilterPlaceholder,
    tableEmptyFilterMessage
  }: Props = $props();

  let metricMode: CoreMetricMode = $state('temperature');
</script>

<div class="tw-ds tw-cpu-screen">
  <h2 class="value-md" style:color="var(--text-primary)">{title}</h2>

  <section>
    <h3 class="label section-title" style:color="var(--text-secondary)">{topologyLabel}</h3>
    <CpuTopologyMap
      {groups}
      {metricMode}
      {metricModeLabel}
      {temperatureModeLabel}
      {clockModeLabel}
      {tooltipLabel}
      onMetricModeChange={(m) => (metricMode = m)}
      {selectedCoreId}
      {onSelectCore}
      {coverageTone}
      {coverageTitle}
      {coverageDescription}
    />
  </section>

  <section>
    <h3 class="label section-title" style:color="var(--text-secondary)">{tableLabel}</h3>
    <CpuAdvancedTable
      rows={tableRows}
      columnLabels={tableColumnLabels}
      filterLabel={tableFilterLabel}
      filterPlaceholder={tableFilterPlaceholder}
      emptyFilterMessage={tableEmptyFilterMessage}
    />
  </section>
</div>

<style>
  .tw-cpu-screen {
    background: transparent;
    padding: var(--space-6);
    max-width: 860px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-cpu-screen h2 {
    margin: 0;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .section-title {
    margin: 0;
  }
</style>
