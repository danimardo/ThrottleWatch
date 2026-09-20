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
  import {
    commandResponseSchemas,
    type CpuTopology
  } from '../../lib/bridge/schemas';

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
    return value === null ? '—' : `${Math.round(value)}${suffix}`;
  }

  let cores = $derived<CoreReading[]>(
    topology.map((core) => ({
      id: core.id,
      index: core.index,
      group: core.group === 'ungrouped' ? undefined : core.group,
      tone: core.temperature_c === null ? 'unknown' : 'warm',
      temperatureLabel: label(core.temperature_c, ' °C'),
      clockLabel: label(core.clock_mhz, ' MHz'),
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
        temperatureLabel: core.temperatureLabel ?? '—',
        temperatureValue: source?.temperature_c ?? undefined,
        clockLabel: core.clockLabel ?? '—',
        clockValue: source?.clock_mhz ?? undefined,
        loadLabel: label(source?.load_percent ?? null, ' %'),
        loadValue: source?.load_percent ?? undefined,
        throttlingLabel: source?.throttling ? 'Sí' : '—',
        unavailable: core.unavailable
      };
    })
  );
  let noReadings = $derived(
    cores.length === 0 || cores.every((core) => core.unavailable)
  );
  const columnLabels: CoreTableColumnLabels = {
    index: 'Núcleo',
    group: 'Grupo',
    temperature: 'Temperatura',
    clock: 'Frecuencia',
    load: 'Carga',
    throttling: 'Limitación'
  };
</script>

<CpuScreen
  title="CPU"
  topologyLabel="Mapa por núcleo"
  tableLabel="Tabla avanzada"
  {groups}
  metricModeLabel="Métrica"
  temperatureModeLabel="Temperatura"
  clockModeLabel="Frecuencia"
  tooltipLabel={(core) =>
    core.unavailable
      ? `Núcleo ${core.index}: datos no disponibles`
      : `Núcleo ${core.index}: ${core.temperatureLabel ?? '—'} · ${core.clockLabel ?? '—'}`}
  {selectedCoreId}
  onSelectCore={(id) => (selectedCoreId = id)}
  coverageTone={noReadings ? 'warning' : undefined}
  coverageTitle={noReadings ? 'Esperando datos del colector' : undefined}
  coverageDescription={noReadings
    ? 'Los núcleos se mantienen visibles aunque no haya una lectura válida.'
    : undefined}
  tableRows={rows}
  tableColumnLabels={columnLabels}
  tableFilterLabel="Filtrar núcleos"
  tableFilterPlaceholder="Número o grupo"
  tableEmptyFilterMessage="No hay núcleos que coincidan."
/>
