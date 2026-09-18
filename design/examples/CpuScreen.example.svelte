<script lang="ts">
  /**
   * Reference composition exercising every state `CpuScreen` (and the
   * `CpuTopologyMap`/`CpuAdvancedTable`/`CoreCell` it's built from)
   * accepts — NOT a component this system exports. A demo control
   * panel switches between a hybrid P/E/LP topology, a homogeneous
   * one, and a 64-core "many cores" topology (to see the advanced
   * table's virtualization actually windowing rows), plus a switch to
   * simulate partial per-core coverage.
   *
   * See OnboardingFlow.example.svelte's note on `$derived.by` — the
   * same TypeScript narrowing false positive applies to every derived
   * value below that compares `topologyPreset` against a literal.
   */
  import { CpuScreen, SegmentedControl, Switch } from '../components';
  import type { CoreGroup, CoreReading, CoreTableRow } from '../components';

  type TopologyPreset = 'hybrid' | 'homogeneous' | 'many';
  let topologyPreset: TopologyPreset = $state('hybrid');
  let simulatePartial = $state(true);
  let selectedCoreId = $state<string | undefined>(undefined);
  let lastAction = $state('(ninguna todavía)');

  function makeCore(id: string, index: number, group: CoreReading['group'], opts: Partial<CoreReading> = {}): CoreReading {
    const base: CoreReading = {
      id,
      index,
      group,
      temperatureLabel: `${68 + ((index * 7) % 24)}°`,
      clockLabel: `${(2.4 + ((index * 3) % 20) / 10).toFixed(1)} GHz`,
      tone: 'normal'
    };
    return { ...base, ...opts };
  }

  let groups: CoreGroup[] = $derived.by(() => {
    if (topologyPreset === 'homogeneous') {
      const cores = Array.from({ length: 16 }, (_, i) =>
        makeCore(`c${i}`, i, undefined, i === 3 && simulatePartial ? { unavailable: true, tone: 'unknown' } : i === 7 ? { throttling: true, tone: 'thermal', temperatureLabel: '97°' } : {})
      );
      return [{ kind: 'ungrouped', cores }];
    }
    if (topologyPreset === 'many') {
      const cores = Array.from({ length: 64 }, (_, i) =>
        makeCore(`m${i}`, i, undefined, simulatePartial && i % 17 === 0 ? { unavailable: true, tone: 'unknown' } : i % 23 === 0 ? { throttling: true, tone: 'thermal' } : {})
      );
      return [{ kind: 'ungrouped', cores }];
    }
    // hybrid
    const p = Array.from({ length: 6 }, (_, i) =>
      makeCore(`p${i}`, i, 'p', i === 2 ? { throttling: true, tone: 'thermal', temperatureLabel: '98°' } : { tone: 'normal' })
    );
    const e = Array.from({ length: 8 }, (_, i) =>
      makeCore(`e${i}`, i, 'e', i === 5 && simulatePartial ? { unavailable: true, tone: 'unknown' } : { tone: 'normal' })
    );
    const lp = Array.from({ length: 2 }, (_, i) => makeCore(`lp${i}`, i, 'lp', { tone: 'normal', clockLabel: '0.9 GHz' }));
    return [
      { kind: 'p', label: 'Núcleos de rendimiento (P)', cores: p },
      { kind: 'e', label: 'Núcleos de eficiencia (E)', cores: e },
      { kind: 'lp', label: 'Núcleos de bajo consumo (LP)', cores: lp }
    ];
  });

  let tableRows: CoreTableRow[] = $derived.by(() =>
    groups.flatMap((g) =>
      g.cores.map((c) => ({
        id: c.id,
        index: c.index,
        groupLabel: g.kind === 'ungrouped' ? '' : g.kind.toUpperCase(),
        temperatureLabel: c.unavailable ? '—' : (c.temperatureLabel ?? '—'),
        temperatureValue: c.unavailable ? undefined : Number((c.temperatureLabel ?? '0').replace('°', '')),
        clockLabel: c.unavailable ? '—' : (c.clockLabel ?? '—'),
        clockValue: c.unavailable ? undefined : Number((c.clockLabel ?? '0').replace(' GHz', '')),
        loadLabel: c.unavailable ? '—' : `${40 + ((c.index * 11) % 55)}%`,
        loadValue: c.unavailable ? undefined : 40 + ((c.index * 11) % 55),
        throttlingLabel: c.unavailable ? '—' : c.throttling ? 'Sí' : 'No',
        unavailable: c.unavailable
      }))
    )
  );

  function tooltipLabel(core: CoreReading): string {
    if (core.unavailable) return `Núcleo ${core.index} · sin datos disponibles`;
    const parts = [core.temperatureLabel, core.clockLabel, core.throttling ? 'throttling activo' : 'sin throttling'];
    return `Núcleo ${core.index} · ${parts.join(' · ')}`;
  }
</script>

<div class="tw-ds cpu-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">TOPOLOGÍA</span>
      <SegmentedControl
        label="Topología"
        value={topologyPreset}
        onchange={(v) => (topologyPreset = v as TopologyPreset)}
        options={[
          { value: 'hybrid', label: 'Híbrida' },
          { value: 'homogeneous', label: 'Homogénea' },
          { value: 'many', label: '64 núcleos' }
        ]}
      />
    </div>
    <div class="control">
      <Switch bind:checked={simulatePartial} label="Simular cobertura parcial" />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <CpuScreen
      title="CPU"
      topologyLabel="Mapa de núcleos"
      tableLabel="Tabla avanzada"
      {groups}
      metricModeLabel="Métrica mostrada"
      temperatureModeLabel="Temperatura"
      clockModeLabel="Reloj"
      {tooltipLabel}
      {selectedCoreId}
      onSelectCore={(id) => {
        selectedCoreId = id;
        lastAction = 'seleccionar núcleo ' + id;
      }}
      coverageTone={simulatePartial ? 'warning' : undefined}
      coverageTitle={simulatePartial ? 'Cobertura parcial de núcleos' : undefined}
      coverageDescription={simulatePartial ? 'Algunos núcleos no reportan datos. Puede deberse a un sensor no disponible en este hardware.' : undefined}
      {tableRows}
      tableColumnLabels={{ index: 'Núcleo', group: 'Grupo', temperature: 'Temp.', clock: 'Reloj', load: 'Carga', throttling: 'Throttling' }}
      tableFilterLabel="Filtrar"
      tableFilterPlaceholder="Núcleo o grupo…"
      tableEmptyFilterMessage="Ningún núcleo coincide con el filtro."
    />
  </div>
</div>

<style>
  .cpu-demo {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .control-panel {
    width: 220px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    position: sticky;
    top: var(--space-6);
  }
  .control {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .last-action {
    padding-top: var(--space-2);
    border-top: 1px solid var(--hairline);
  }
  .screen-frame {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  @media (max-width: 900px) {
    .cpu-demo {
      flex-direction: column;
    }
    .control-panel {
      width: auto;
      position: static;
    }
    .screen-frame {
      width: 100%;
    }
  }
</style>
