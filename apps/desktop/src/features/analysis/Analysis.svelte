<script lang="ts">
  import { onMount } from 'svelte';
  import AnalysisScreen from '../../design-system/components/AnalysisScreen.svelte';
  import Button from '../../design-system/components/Button.svelte';
  import ExportDialog from '../../design-system/components/ExportDialog.svelte';
  import {
    commandResponseSchemas,
    type AnalysisWindow,
    type ExportPreview
  } from '../../lib/bridge/schemas';
  import { invokeValidated } from '../../lib/bridge';
  import { createTranslator } from '../../lib/i18n';
  import { toAnalysisView } from './model';

  const { t } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );

  let status = $state<'noHistory' | 'loading' | 'ready'>('loading');
  let window = $state<AnalysisWindow | null>(null);
  let selectedEventId = $state<string | undefined>();
  let selectedRange = $state<[number, number] | null>(null);
  let exportOpen = $state(false);
  let exportFormat = $state<'csv' | 'json'>('csv');
  let anonymize = $state(true);
  let exportPreview = $state<ExportPreview | null>(null);
  let exportBusy = $state(false);

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
          title: t('analysis.selectedEvent'),
          description: selectedEventId,
          items: [
            { label: t('analysis.source'), value: t('analysis.limitEvent') }
          ]
        }
      : undefined
  );
  let rangeEvidence = $derived(
    selectedRange
      ? {
          title: t('analysis.selectedRange'),
          description: `${selectedRange[0]}–${selectedRange[1]} ms`,
          items: [{ label: t('common.resolution'), value: t('common.high') }]
        }
      : undefined
  );

  function selectRange(range: [number, number] | null): void {
    selectedRange = range;
    if (range) void loadWindow(range[0], Math.max(range[1], range[0] + 1));
  }

  async function refreshExportPreview(): Promise<void> {
    if (selectedRange === null) {
      exportPreview = null;
      return;
    }
    const result = await invokeValidated(
      'preview_export',
      {
        request: {
          scope: {
            kind: 'range',
            session_id: 'latest',
            start_ms: selectedRange[0],
            end_ms: selectedRange[1]
          },
          format: exportFormat,
          anonymize
        }
      },
      commandResponseSchemas.preview_export
    );
    exportPreview = result.ok ? result.value : null;
  }

  async function openExport(): Promise<void> {
    exportOpen = true;
    await refreshExportPreview();
  }

  async function runExport(): Promise<void> {
    if (selectedRange === null) return;
    exportBusy = true;
    const result = await invokeValidated(
      'export',
      {
        request: {
          scope: {
            kind: 'range',
            session_id: 'latest',
            start_ms: selectedRange[0],
            end_ms: selectedRange[1]
          },
          format: exportFormat,
          anonymize
        }
      },
      commandResponseSchemas.export
    );
    exportBusy = false;
    if (result.ok) exportOpen = false;
  }

  function cancelExport(): void {
    if (exportBusy)
      void invokeValidated(
        'cancel_export',
        undefined,
        commandResponseSchemas.cancel_export
      );
    exportOpen = false;
  }
</script>

<div class="analysis-actions">
  <Button
    variant="secondary"
    label={t('analysis.exportRange')}
    disabled={!selectedRange}
    onclick={() => void openExport()}
  />
</div>

<AnalysisScreen
  {status}
  loadingLabel={t('analysis.loading')}
  noHistoryTitle={t('analysis.noHistory')}
  noHistoryDescription={t('analysis.noHistoryDescription')}
  tracks={view.tracks}
  events={view.events}
  {selectedEventId}
  onSelectEvent={(id) => (selectedEventId = id)}
  onRangeSelect={selectRange}
  valueLabel={(point) =>
    point?.value === null || point === undefined
      ? t('analysis.noData')
      : String(point.value)}
  timeLabel={(time) => `${Math.round(time / 1000)} s`}
  legendLabel={t('analysis.tracksLabel')}
  cursorLabel={t('analysis.cursorLabel')}
  rangeSummaryLabel={(range) =>
    `${t('analysis.range')} ${range[0]}–${range[1]}`}
  resetRangeLabel={t('analysis.clearRange')}
  {selectedEventEvidence}
  {rangeEvidence}
  idleHint={t('analysis.selectEvidence')}
  evidenceTitle={t('analysis.evidence')}
/>

<ExportDialog
  bind:open={exportOpen}
  title={t('analysis.export')}
  scopeLabel={selectedRange
    ? `${t('analysis.range')} ${selectedRange[0]}–${selectedRange[1]} ms`
    : t('common.notSelected')}
  formatLabel={t('analysis.format')}
  format={exportFormat}
  formatOptions={[
    { value: 'csv', label: t('analysis.csv') },
    { value: 'json', label: t('analysis.json') }
  ]}
  onFormatChange={(value) => {
    exportFormat = value;
    void refreshExportPreview();
  }}
  anonymizeLabel={t('analysis.anonymize')}
  anonymizeDescription={t('analysis.anonymizeDescription')}
  {anonymize}
  onAnonymizeChange={(value) => {
    anonymize = value;
    void refreshExportPreview();
  }}
  includedTitle={t('analysis.included')}
  includedFields={exportPreview?.included_fields ?? []}
  excludedTitle={t('analysis.excluded')}
  excludedFields={exportPreview?.excluded_fields ?? []}
  sizeLabel={exportPreview
    ? `${t('analysis.estimatedSize')}: ${exportPreview.estimated_bytes} B`
    : undefined}
  fileNameLabel={exportPreview?.proposed_file_name}
  cancelLabel={t('analysis.cancel')}
  confirmLabel={exportBusy ? t('analysis.loading') : t('analysis.save')}
  confirmDisabled={exportPreview === null || exportBusy}
  onCancel={cancelExport}
  onConfirm={() => void runExport()}
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
