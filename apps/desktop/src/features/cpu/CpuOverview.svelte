<script lang="ts">
  import { onMount } from 'svelte';
  import CpuScreen from '../../design-system/components/CpuScreen.svelte';
  import type { CoreGroup } from '../../design-system/components/CpuTopologyMap.svelte';
  import type { CoreReading } from '../../design-system/components/CoreCell.svelte';
  import type {
    CoreTableColumnLabels,
    CoreTableRow
  } from '../../design-system/components/CpuAdvancedTable.svelte';
  import { invokeValidated } from '../../lib/bridge';
  import { coreTemperatureView } from './model';
  import { getTranslator } from '../../lib/i18n/runtime';
  import {
    commandResponseSchemas,
    liveSnapshotSchema,
    type CpuTopology
  } from '../../lib/bridge/schemas';

  const { t } = getTranslator();

  let topology = $state<CpuTopology['cores']>([]);
  let generalTemperatureC = $state<number | null>(null);
  let selectedCoreId = $state<string | undefined>();

  async function loadTopology(): Promise<void> {
    const result = await invokeValidated(
      'get_cpu_topology',
      undefined,
      commandResponseSchemas.get_cpu_topology
    );
    if (result.ok) topology = result.value.cores;
    const live = await invokeValidated(
      'get_live_snapshot',
      undefined,
      liveSnapshotSchema
    );
    if (live.ok) generalTemperatureC = live.value.temperature_c;
  }

  // The readings used to be fetched once when the screen opened and then never again, so the
  // figures were frozen at whatever the first sample said.
  onMount(() => {
    void loadTopology();
    const timer = setInterval(() => void loadTopology(), 2000);
    return () => clearInterval(timer);
  });

  function label(value: number | null, suffix: string): string {
    return value === null
      ? t('common.noValue')
      : `${Math.round(value)}${suffix}`;
  }

  let cores = $derived<CoreReading[]>(
    topology.map((core) => ({
      id: core.id,
      index: core.index,
      group: core.group === 'ungrouped' ? undefined : core.group,
      tone:
        core.temperature_c === null && generalTemperatureC === null
          ? 'unknown'
          : 'warm',
      temperatureLabel: coreTemperatureView(
        t,
        core.temperature_c,
        generalTemperatureC
      ).label,
      clockLabel: label(core.clock_mhz, ` ${t('dashboard.mhz')}`),
      throttling: core.throttling === true,
      unavailable: core.temperature_c === null && core.clock_mhz === null
    }))
  );
  let groups = $derived<CoreGroup[]>(
    ['p', 'e', 'lp', 'ungrouped']
      .map((kind) => ({
        kind: kind as CoreGroup['kind'],
        label: kind === 'ungrouped' ? undefined : kind.toUpperCase(),
        cores: cores.filter((core) => (core.group ?? 'ungrouped') === kind)
      }))
      .filter((group) => group.cores.length > 0)
  );
  let rows = $derived<CoreTableRow[]>(
    cores.map((core) => {
      const source = topology[core.index];
      return {
        id: core.id,
        index: core.index,
        groupLabel: core.group?.toUpperCase() ?? '',
        temperatureLabel: core.temperatureLabel ?? t('common.noValue'),
        temperatureValue: source?.temperature_c ?? generalTemperatureC ?? undefined,
        clockLabel: core.clockLabel ?? t('common.noValue'),
        clockValue: source?.clock_mhz ?? undefined,
        loadLabel: label(
          source?.load_percent ?? null,
          ` ${t('dashboard.percent')}`
        ),
        loadValue: source?.load_percent ?? undefined,
        throttlingLabel: source?.throttling
          ? t('common.yes')
          : t('common.noValue'),
        unavailable: core.unavailable
      };
    })
  );
  let noReadings = $derived(
    cores.length === 0 || cores.every((core) => core.unavailable)
  );
  // Some processors (this Ryzen among them) report one temperature for the whole chip and none per
  // core: say so, instead of a grid of dashes that reads as broken.
  let onlyChipTemperature = $derived(
    !noReadings &&
      generalTemperatureC !== null &&
      topology.every((core) => core.temperature_c === null)
  );
  let someCoresHaveNoReading = $derived(
    !noReadings && cores.some((core) => core.unavailable)
  );
  const columnLabels: CoreTableColumnLabels = {
    index: t('cpu.core'),
    group: t('cpu.group'),
    temperature: t('cpu.temperature'),
    clock: t('cpu.clock'),
    load: t('cpu.load'),
    throttling: t('cpu.throttling')
  };
</script>

<CpuScreen
  title={t('cpu.title')}
  topologyLabel={t('cpu.topology')}
  tableLabel={t('cpu.table')}
  {groups}
  metricModeLabel={t('cpu.metric')}
  temperatureModeLabel={t('cpu.temperatureMode')}
  clockModeLabel={t('cpu.clockMode')}
  tooltipLabel={(core) =>
    core.unavailable
      ? t('cpu.unavailableCore').replace('{index}', String(core.index))
      : t('cpu.coreSummary')
          .replace('{index}', String(core.index))
          .replace(
            '{temperature}',
            core.temperatureLabel ?? t('common.noValue')
          )
          .replace('{clock}', core.clockLabel ?? t('common.noValue'))}
  {selectedCoreId}
  onSelectCore={(id) => (selectedCoreId = id)}
  coverageTone={noReadings ? 'warning' : onlyChipTemperature ? 'info' : undefined}
  coverageTitle={noReadings
    ? t('cpu.waiting')
    : onlyChipTemperature
      ? t('cpu.chipTemperatureOnlyTitle')
      : undefined}
  coverageDescription={noReadings
    ? t('cpu.waitingDescription')
    : onlyChipTemperature
      ? t('cpu.chipTemperatureOnlyDescription')
      : undefined}
  tableRows={rows}
  tableColumnLabels={columnLabels}
  tableFilterLabel={t('cpu.filter')}
  tableFilterPlaceholder={t('cpu.filterPlaceholder')}
  tableEmptyFilterMessage={t('cpu.emptyFilter')}
  topologyHelp={someCoresHaveNoReading ? t('help.hatched') : undefined}
  temperatureHelp={onlyChipTemperature ? t('help.coreTemperature') : undefined}
  throttlingHelp={t('help.throttling')}
  helpLabel={t('help.more')}
/>
