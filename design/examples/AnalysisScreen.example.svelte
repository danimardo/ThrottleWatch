<script lang="ts">
  /**
   * Reference composition exercising every state `AnalysisScreen` (and
   * the `AnalysisChart` it wraps) accepts — NOT a component this system
   * exports. Synthetic data is built once with `buildTracks()`: four
   * tracks (temperatura, reloj, carga, potencia) over 120 samples, with
   * a real gap (`value: null`, indices 40–45) in the temperature track,
   * a reduced-quality run (indices 70–80) in the load track, and three
   * events — one thermal, one electrical, one mixed — so every visual
   * rule the chart draws (gap break, dashed reduced-quality segment,
   * solid vs. striped event band) actually appears somewhere in the
   * demo.
   *
   * The evidence objects (`selectedEventEvidence`/`rangeEvidence`) are
   * computed HERE, in the demo, from the synthetic samples — this is
   * exactly the "host does the lookup" responsibility the component's
   * own doc comment describes; `AnalysisScreen` itself never touches
   * `events` to find one by id.
   *
   * See OnboardingFlow.example.svelte's note on `$derived.by` — applied
   * here for every derived value compared against a `$state`-declared
   * literal later in the same scope.
   */
  import { AnalysisScreen, SegmentedControl, Switch, Select } from '../components';
  import type {
    AnalysisTrack,
    AnalysisEvent,
    AnalysisPoint,
    AnalysisStatus,
    AnalysisEvidence
  } from '../components';

  // All three of these are exactly the kind of copy the real app pulls
  // from its own ES/EN catalog (see AnalysisChart.svelte's doc comment
  // for why the component itself takes callbacks instead of baking any
  // of this in) — hardcoded here only because this file is a demo.
  function valueLabel(point: AnalysisPoint | undefined, track: AnalysisTrack): string {
    if (!point || point.value === null) return 'sin dato';
    const suffix = point.reducedQuality ? ' (calidad reducida)' : '';
    return `${point.value}${track.unit ?? ''}${suffix}`;
  }
  function rangeSummaryLabel(range: [number, number]): string {
    const [lo, hi] = range;
    return `Rango seleccionado: muestras ${lo}–${hi} (${hi - lo + 1} muestras)`;
  }
  const cursorLabel = 'Cursor del gráfico y selección de rango';
  const resetRangeLabel = 'Restablecer zoom ×';

  const SAMPLE_COUNT = 120;

  function buildTracks(): AnalysisTrack[] {
    const temperature: AnalysisTrack = {
      kind: 'temperature',
      label: 'Temperatura',
      unit: '°C',
      tone: 'thermal',
      min: 40,
      max: 100,
      points: Array.from({ length: SAMPLE_COUNT }, (_, t) => {
        if (t >= 40 && t <= 45) return { t, value: null };
        const spike = t >= 20 && t <= 35 ? 22 : 0;
        return { t, value: 58 + spike + Math.round(6 * Math.sin(t / 4)) };
      })
    };
    const clock: AnalysisTrack = {
      kind: 'clock',
      label: 'Frecuencia activa',
      unit: ' GHz',
      tone: 'accent',
      min: 0.8,
      max: 4.5,
      points: Array.from({ length: SAMPLE_COUNT }, (_, t) => {
        const dip = t >= 20 && t <= 35 ? -1.4 : 0;
        return { t, value: Number((3.2 + dip + 0.3 * Math.sin(t / 5)).toFixed(2)) };
      })
    };
    const load: AnalysisTrack = {
      kind: 'load',
      label: 'Carga',
      unit: '%',
      tone: 'normal',
      min: 0,
      max: 100,
      points: Array.from({ length: SAMPLE_COUNT }, (_, t) => ({
        t,
        value: Math.max(5, Math.min(100, 55 + Math.round(30 * Math.sin(t / 6)))),
        reducedQuality: t >= 70 && t <= 80
      }))
    };
    const power: AnalysisTrack = {
      kind: 'power',
      label: 'Potencia',
      unit: ' W',
      tone: 'power',
      min: 5,
      max: 65,
      points: Array.from({ length: SAMPLE_COUNT }, (_, t) => {
        const spike = t >= 60 && t <= 68 ? 18 : 0;
        return { t, value: 28 + spike + Math.round(5 * Math.sin(t / 3)) };
      })
    };
    return [temperature, clock, load, power];
  }

  const tracks = buildTracks();

  const events: AnalysisEvent[] = [
    { id: 'ev-turbo', kind: 'info', startT: 12, endT: 12, label: 'Fin del turbo de potencia (esperado)' },
    { id: 'ev-thermal', kind: 'thermal', startT: 20, endT: 35, label: 'Limitación térmica confirmada' },
    { id: 'ev-electrical', kind: 'electrical', startT: 60, endT: 68, label: 'Limitación por potencia' },
    { id: 'ev-mixed', kind: 'mixed', startT: 90, endT: 100, label: 'Térmica y potencia a la vez' },
    { id: 'ev-platform', kind: 'platform', startT: 106, endT: 118, label: 'Limitada por el equipo (el fabricante bajó la potencia)' }
  ];

  const EVENT_EVIDENCE: Record<string, AnalysisEvidence> = {
    'ev-turbo': {
      title: 'Fin del turbo de potencia (muestra 12)',
      description: 'La potencia bajó de 64 W a 45 W en 2 s con la carga constante y 18 °C de margen: terminó la ventana de turbo (PL2 → PL1). Es el comportamiento previsto del procesador, no una limitación.',
      items: [
        { label: 'Potencia', value: '64 → 45 W' },
        { label: 'Margen térmico', value: '18 °C' }
      ]
    },
    'ev-thermal': {
      title: 'Limitación térmica confirmada (muestras 20–35)',
      description: 'El procesador declaró la razón THERMAL en el 73 % de las muestras, con la potencia por debajo de su límite. La frecuencia activa quedó por debajo de la base: throttling real, no boost oportunista.',
      items: [
        { label: 'Razón THERMAL', value: '73 % de las muestras' },
        { label: 'Frecuencia activa', value: '1,8 GHz (base 2,6 GHz)' },
        { label: 'Potencia', value: '31 W de 45 W' },
        { label: 'Enfriar mejor', value: '+10–20 % (notable)' }
      ]
    },
    'ev-electrical': {
      title: 'Limitación por potencia (muestras 60–68)',
      description: 'La potencia se mantuvo plana en su límite (45 W) con 14 °C de margen térmico: la refrigeración apenas influiría.',
      items: [
        { label: 'Razón PL1', value: '88 % de las muestras' },
        { label: 'Margen térmico', value: '14 °C' }
      ]
    },
    'ev-mixed': {
      title: 'Térmica y potencia a la vez (muestras 90–100)',
      description: 'Las razones THERMAL y PL1 aparecen a la vez en esta ventana; no se fuerza una causa única.',
      items: [{ label: 'Duración', value: '10 muestras' }]
    },
    'ev-platform': {
      title: 'Limitada por el equipo (muestras 106–118)',
      description: 'El límite de potencia efectivo bajó de 45 W a 28 W sin cambio de plan ni de alimentación y con la CPU a 74 °C: el fabricante reduce la potencia por el calor del chasis. Mejorar la ventilación del equipo sí ayudaría.',
      items: [
        { label: 'Límite de potencia', value: '45 → 28 W' },
        { label: 'Temperatura CPU', value: '74 °C (margen 26 °C)' }
      ]
    }
  };

  type StatusPreset = AnalysisStatus;
  let statusPreset: StatusPreset = $state('ready');
  let dataNotice = $state<'none' | 'stale' | 'partial'>('none');

  let selectedEventId = $state<string | undefined>(undefined);
  let rangeEvidence = $state<AnalysisEvidence | undefined>(undefined);
  let lastAction = $state('(ninguna todavía)');

  let selectedEventEvidence = $derived.by(() =>
    selectedEventId ? EVENT_EVIDENCE[selectedEventId] : undefined
  );

  function onSelectEvent(id: string | undefined) {
    selectedEventId = id;
    lastAction = id ? `seleccionar evento ${id}` : 'deseleccionar evento';
    if (id) rangeEvidence = undefined;
  }

  function onRangeSelect(range: [number, number] | null) {
    if (!range) {
      rangeEvidence = undefined;
      lastAction = 'limpiar selección de rango';
      return;
    }
    selectedEventId = undefined;
    const [lo, hi] = range;
    const tempPoints = tracks[0].points.slice(lo, hi + 1).map((p) => p.value).filter((v): v is number => v !== null);
    const avgTemp = tempPoints.length ? Math.round(tempPoints.reduce((a, b) => a + b, 0) / tempPoints.length) : undefined;
    rangeEvidence = {
      title: `Rango seleccionado (muestras ${lo}–${hi})`,
      description: 'Promedio calculado sobre las muestras dentro de este rango.',
      items: [
        { label: 'Muestras', value: `${hi - lo + 1}` },
        ...(avgTemp !== undefined ? [{ label: 'Temp. media', value: `${avgTemp}°C` }] : [])
      ]
    };
    lastAction = `seleccionar rango ${lo}–${hi}`;
  }

  let dataNoticeTitle = $derived.by(() => {
    if (dataNotice === 'stale') return 'Datos obsoletos';
    if (dataNotice === 'partial') return 'Datos parciales';
    return undefined;
  });
  let dataNoticeDescription = $derived.by(() => {
    if (dataNotice === 'stale') return 'La última lectura tiene más de 10 minutos de antigüedad.';
    if (dataNotice === 'partial') return 'Algunos sensores no reportaron datos durante parte de esta sesión.';
    return undefined;
  });
</script>

<div class="tw-ds analysis-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">ESTADO</span>
      <SegmentedControl
        label="Estado"
        value={statusPreset}
        onchange={(v) => (statusPreset = v as StatusPreset)}
        options={[
          { value: 'noHistory', label: 'Sin historial' },
          { value: 'loading', label: 'Cargando' },
          { value: 'ready', label: 'Listo' }
        ]}
      />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">AVISO DE DATOS</span>
      <Select
        label="Aviso de datos"
        value={dataNotice}
        onchange={(v) => (dataNotice = v as typeof dataNotice)}
        options={[
          { value: 'none', label: 'Ninguno' },
          { value: 'stale', label: 'Obsoletos' },
          { value: 'partial', label: 'Parciales' }
        ]}
      />
    </div>
    <div class="control">
      <Switch
        checked={selectedEventId === 'ev-thermal'}
        onchange={(checked) => onSelectEvent(checked ? 'ev-thermal' : undefined)}
        label="Simular clic en evento térmico"
      />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <AnalysisScreen
      status={statusPreset}
      loadingLabel="Cargando historial…"
      noHistoryTitle="Todavía no hay historial suficiente"
      noHistoryDescription="Ejecuta al menos una sesión de diagnóstico para ver el análisis aquí."
      {dataNoticeTitle}
      {dataNoticeDescription}
      {tracks}
      {events}
      {selectedEventId}
      {onSelectEvent}
      {valueLabel}
      timeLabel={(t) => `m${t}`}
      legendLabel="Pistas de la sesión"
      {cursorLabel}
      {rangeSummaryLabel}
      {resetRangeLabel}
      {onRangeSelect}
      {selectedEventEvidence}
      {rangeEvidence}
      idleHint="Pasa el cursor sobre el gráfico, selecciona un evento o arrastra en la franja inferior para ver evidencia aquí."
      evidenceTitle="Evidencia"
    />
  </div>
</div>

<style>
  .analysis-demo {
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
    .analysis-demo {
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
