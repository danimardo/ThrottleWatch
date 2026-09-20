<script lang="ts">
  import { onMount } from 'svelte';
  import AnalysisScreen from '../../design-system/components/AnalysisScreen.svelte';
  import Button from '../../design-system/components/Button.svelte';
  import ExportDialog from '../../design-system/components/ExportDialog.svelte';
  import {
    commandResponseSchemas,
    type AnalysisWindow
  } from '../../lib/bridge/schemas';
  import { invokeValidated } from '../../lib/bridge';
  import { toAnalysisView } from './model';

  let status = $state<'noHistory' | 'loading' | 'ready'>('loading');
  let window = $state<AnalysisWindow | null>(null);
  let selectedEventId = $state<string | undefined>();
  let selectedRange = $state<[number, number] | null>(null);
  let exportOpen = $state(false);
  let exportFormat = $state<'csv' | 'json'>('csv');
  let anonymize = $state(true);

  async function loadWindow(startMs = 0, endMs = 86_400_000): Promise<void> {
    // Keep the chart mounted while a zoom asks for a higher-resolution window.
    // Otherwise AnalysisChart loses its keyboard range anchor/selection during
    // the request and the user sees the evidence panel disappear briefly.
    if (window === null) status = 'loading';
    const result = await invokeValidated(
      'get_analysis_window',
      {
        request: {
          session_id: 'latest',
          start_ms: startMs,
          end_ms: endMs,
          target_points_per_track: 3000
        }
      },
      commandResponseSchemas.get_analysis_window
    );
    if (result.ok && result.value.tracks.length > 0) {
      window = result.value;
      status = 'ready';
    } else status = 'noHistory';
  }

  onMount(() => void loadWindow());

  let view = $derived(
    window ? toAnalysisView(window) : { tracks: [], events: [] }
  );
  let selectedEventEvidence = $derived(
    selectedEventId
      ? {
          title: 'Evento seleccionado',
          description: selectedEventId,
          items: [{ label: 'Origen', value: 'limit_event' }]
        }
      : undefined
  );
  let rangeEvidence = $derived(
    selectedRange
      ? {
          title: 'Rango seleccionado',
          description: `${selectedRange[0]}–${selectedRange[1]} ms`,
          items: [{ label: 'Resolución', value: 'alta' }]
        }
      : undefined
  );

  function selectRange(range: [number, number] | null): void {
    selectedRange = range;
    if (range) void loadWindow(range[0], Math.max(range[1], range[0] + 1));
  }
</script>

<div class="analysis-actions">
  <Button
    variant="secondary"
    label="Exportar rango"
    disabled={!selectedRange}
    onclick={() => (exportOpen = true)}
  />
</div>

<AnalysisScreen
  {status}
  loadingLabel="Cargando análisis"
  noHistoryTitle="No hay historial suficiente"
  noHistoryDescription="Completa una sesión para poder analizarla."
  tracks={view.tracks}
  events={view.events}
  {selectedEventId}
  onSelectEvent={(id) => (selectedEventId = id)}
  onRangeSelect={selectRange}
  valueLabel={(point) =>
    point?.value === null || point === undefined
      ? 'Sin datos'
      : String(point.value)}
  timeLabel={(time) => `${Math.round(time / 1000)} s`}
  legendLabel="Pistas del análisis"
  cursorLabel="Cursor temporal"
  rangeSummaryLabel={(range) => `Rango ${range[0]}–${range[1]}`}
  resetRangeLabel="Borrar rango"
  {selectedEventEvidence}
  {rangeEvidence}
  idleHint="Selecciona un evento o un rango para ver la evidencia."
  evidenceTitle="Evidencia"
/>

<ExportDialog
  bind:open={exportOpen}
  title="Exportar análisis"
  scopeLabel={selectedRange
    ? `Rango ${selectedRange[0]}–${selectedRange[1]} ms`
    : 'Rango no seleccionado'}
  formatLabel="Formato"
  format={exportFormat}
  formatOptions={[
    { value: 'csv', label: 'CSV' },
    { value: 'json', label: 'JSON' }
  ]}
  onFormatChange={(value) => (exportFormat = value)}
  anonymizeLabel="Anonimizar"
  anonymizeDescription="Oculta identificadores del equipo en la exportación."
  {anonymize}
  onAnonymizeChange={(value) => (anonymize = value)}
  includedTitle="Incluye"
  includedFields={['muestras', 'calidad', 'eventos']}
  excludedTitle="Excluye"
  excludedFields={[]}
  sizeLabel="Tamaño estimado: calculado al guardar"
  fileNameLabel="throttlewatch_analysis.csv"
  cancelLabel="Cancelar"
  confirmLabel="Guardar"
  onCancel={() => (exportOpen = false)}
  onConfirm={() => (exportOpen = false)}
/>

<style>
  .analysis-actions {
    display: flex;
    justify-content: flex-end;
    max-width: 1100px;
    margin: var(--space-4) auto 0;
    padding: 0 var(--space-6);
  }
</style>
