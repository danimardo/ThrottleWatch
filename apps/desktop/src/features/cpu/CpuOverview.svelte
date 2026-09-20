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
  import { createTranslator } from '../../lib/i18n';
  import {
    commandResponseSchemas,
    type CpuTopology
  } from '../../lib/bridge/schemas';

  const { t } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );

  let topology = $state<CpuTopology['cores']>([]);
  let selectedCoreId = $state<string | undefined>();

  onMount(() => {
    void invokeValidated(
      'get_cpu_topology',
      undefined,
      commandResponseSchemas.get_cpu_topology
    ).then((result) => {
      if (result.ok) topology = result.value.cores;
    });
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
      tone: core.temperature_c === null ? 'unknown' : 'warm',
      temperatureLabel: label(core.temperature_c, ` ${t('dashboard.celsius')}`),
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
        temperatureValue: source?.temperature_c ?? undefined,
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
  coverageTone={noReadings ? 'warning' : undefined}
  coverageTitle={noReadings ? t('cpu.waiting') : undefined}
  coverageDescription={noReadings ? t('cpu.waitingDescription') : undefined}
  tableRows={rows}
  tableColumnLabels={columnLabels}
  tableFilterLabel={t('cpu.filter')}
  tableFilterPlaceholder={t('cpu.filterPlaceholder')}
  tableEmptyFilterMessage={t('cpu.emptyFilter')}
/>
