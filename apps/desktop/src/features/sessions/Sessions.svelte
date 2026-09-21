<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../design-system/components/Button.svelte';
  import ExportDialog, {
    type ExportFormat
  } from '../../design-system/components/ExportDialog.svelte';
  import ImportResultDialog, {
    type ImportStatus
  } from '../../design-system/components/ImportResultDialog.svelte';
  import ReportScreen from '../../design-system/components/ReportScreen.svelte';
  import SessionsScreen, {
    type SessionsScreenSession
  } from '../../design-system/components/SessionsScreen.svelte';
  import type { Classification } from '../../design-system/lib/classification';
  import {
    invokeValidated,
    listenValidated,
    parseEvent
  } from '../../lib/bridge';
  import {
    commandResponseSchemas,
    type ExportPreview,
    type SessionSummary
  } from '../../lib/bridge/schemas';
  import { createTranslator } from '../../lib/i18n';

  const { t, locale } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );

  const classifications: readonly Classification[] = [
    'normal',
    'hot_unproven',
    'thermal_probable',
    'thermal_confirmed',
    'power_limited',
    'platform_limited',
    'mixed_limit',
    'indeterminate'
  ];

  function isClassification(value: unknown): value is Classification {
    return (
      typeof value === 'string' &&
      classifications.some((item) => item === value)
    );
  }

  let listStatus = $state<'loading' | 'error' | 'loaded'>('loading');
  let sessions = $state<SessionSummary[]>([]);
  let view = $state<'list' | 'report'>('list');
  let selectedSession = $state<SessionSummary | null>(null);
  let selectedReport = $state<Record<string, unknown> | null>(null);
  let reevaluatedReport = $state<Record<string, unknown> | null>(null);
  let reevaluationUnavailable = $state(false);
  let reportStatus = $state<'loading' | 'ready'>('ready');
  let reportUnavailable = $state(false);
  let exportOpen = $state(false);
  let exportScope = $state<'session' | 'report'>('session');
  let exportFormat = $state<ExportFormat>('json');
  let exportAnonymize = $state(true);
  let exportPreview = $state<ExportPreview | null>(null);
  let exportBusy = $state(false);
  let importOpen = $state(false);
  let importStatus = $state<ImportStatus>('reading');
  let importDescription = $state<string | undefined>();
  let importedSessionId = $state<string | undefined>();
  let openExportWhenReady = $state(false);

  const dateFormatter = new Intl.DateTimeFormat(
    locale === 'es' ? 'es-ES' : 'en-US',
    {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }
  );

  function classificationOf(value: unknown): Classification {
    return isClassification(value) ? value : 'indeterminate';
  }

  function classificationLabel(value: unknown): string {
    const classification = classificationOf(value);
    const labels: Record<Classification, string> = {
      normal: t('sessions.classificationNormal'),
      hot_unproven: t('sessions.classificationHotUnproven'),
      thermal_probable: t('sessions.classificationThermalProbable'),
      thermal_confirmed: t('sessions.classificationThermalConfirmed'),
      power_limited: t('sessions.classificationPowerLimited'),
      platform_limited: t('sessions.classificationPlatformLimited'),
      mixed_limit: t('sessions.classificationMixedLimit'),
      indeterminate: t('sessions.classificationIndeterminate')
    };
    return labels[classification];
  }

  function typeLabel(kind: SessionSummary['kind']): string {
    return t(
      kind === 'guided'
        ? 'sessions.typeGuided'
        : kind === 'imported'
          ? 'sessions.typeImported'
          : 'sessions.typePassive'
    );
  }

  function statusLabel(status: SessionSummary['status']): string {
    return t(
      `sessions.status${status.charAt(0).toUpperCase()}${status.slice(1)}`
    );
  }

  function dateLabel(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.valueOf()) ? value : dateFormatter.format(date);
  }

  function durationLabel(value: number | null): string | undefined {
    if (value === null || !Number.isFinite(value) || value < 0)
      return undefined;
    return `${Math.round(value / 1000)} ${t('common.seconds')}`;
  }

  function toScreenSession(summary: SessionSummary): SessionsScreenSession {
    const classified = summary.report_classification !== null;
    return {
      id: summary.session_id,
      status: summary.status,
      typeLabel: typeLabel(summary.kind),
      dateLabel: dateLabel(summary.started_at),
      durationLabel: durationLabel(summary.duration_ms),
      statusLabel: classified ? undefined : statusLabel(summary.status),
      classification: classified
        ? classificationOf(summary.report_classification)
        : undefined,
      classificationLabel: classified
        ? classificationLabel(summary.report_classification)
        : undefined,
      importedLabel:
        summary.status === 'imported'
          ? t('sessions.statusImported')
          : undefined,
      isReference: summary.is_reference,
      referenceLabel: summary.is_reference
        ? t('sessions.reference')
        : undefined,
      referenceActionLabel:
        summary.kind === 'guided' && summary.status === 'completed'
          ? summary.is_reference
            ? t('sessions.removeReference')
            : t('sessions.useReference')
          : undefined,
      onToggleReference:
        summary.kind === 'guided' && summary.status === 'completed'
          ? () => void toggleReference(summary)
          : undefined,
      openLabel: t('sessions.open'),
      onOpen: () => void openSession(summary.session_id),
      deleteLabel:
        summary.status === 'active' ? undefined : t('sessions.delete')
    };
  }

  let screenSessions = $derived(sessions.map(toScreenSession));
  let reportClassification = $derived(
    classificationOf(selectedReport?.classification)
  );
  let reportClassificationLabel = $derived(
    classificationLabel(selectedReport?.classification)
  );
  let reevaluatedClassification = $derived(
    reevaluatedReport
      ? classificationOf(reevaluatedReport.classification)
      : null
  );
  let reevaluatedView = $derived(
    reevaluatedReport && reevaluatedClassification
      ? {
          title: t('sessions.reevaluatedTitle'),
          classification: reevaluatedClassification,
          classificationLabel: classificationLabel(
            reevaluatedReport.classification
          ),
          headline:
            reevaluatedClassification === reportClassification
              ? t('sessions.reevaluatedNoNote')
              : t('sessions.reevaluatedHeadline'),
          note:
            reevaluatedClassification === reportClassification
              ? undefined
              : t('sessions.reevaluatedNote')
        }
      : reevaluationUnavailable
        ? {
            title: t('sessions.reevaluatedTitle'),
            classification: 'indeterminate' as const,
            classificationLabel: classificationLabel('indeterminate'),
            headline: t('sessions.reevaluatedUnavailable')
          }
        : undefined
  );
  let reportEvidence = $derived(
    Array.isArray(selectedReport?.events)
      ? selectedReport.events
          .map((event) => {
            if (typeof event !== 'object' || event === null) return null;
            const kind =
              'kind' in event && typeof event.kind === 'string'
                ? event.kind
                : null;
            const start =
              'start_ms' in event && typeof event.start_ms === 'number'
                ? event.start_ms
                : null;
            const end =
              'end_ms' in event && typeof event.end_ms === 'number'
                ? event.end_ms
                : null;
            if (kind === null) return null;
            return end === null || start === null
              ? kind
              : `${kind} · ${Math.max(0, end - start)} ms`;
          })
          .filter((event): event is string => event !== null)
      : []
  );
  let reportImpact = $derived(
    typeof selectedReport?.cooling_potential === 'object' &&
      selectedReport.cooling_potential !== null
      ? JSON.stringify(selectedReport.cooling_potential)
      : undefined
  );

  async function loadSessions(): Promise<void> {
    listStatus = 'loading';
    const result = await invokeValidated(
      'list_sessions',
      { request: { cursor: null, limit: 100 } },
      commandResponseSchemas.list_sessions
    );
    if (!result.ok) {
      listStatus = 'error';
      return;
    }
    sessions = result.value.sessions;
    listStatus = 'loaded';
    if (openExportWhenReady) {
      openExportWhenReady = false;
      const candidate =
        sessions.find((session) => session.status === 'active') ?? sessions[0];
      if (candidate !== undefined) {
        selectedSession = candidate;
        await openExport('session');
      }
    }
  }

  async function openSession(sessionId: string): Promise<void> {
    view = 'report';
    reportStatus = 'loading';
    reportUnavailable = false;
    selectedSession =
      sessions.find((session) => session.session_id === sessionId) ?? null;
    selectedReport = null;
    reevaluatedReport = null;
    reevaluationUnavailable = false;
    const result = await invokeValidated(
      'get_session',
      { request: { session_id: sessionId } },
      commandResponseSchemas.get_session
    );
    if (!result.ok) {
      reportUnavailable = true;
      reportStatus = 'ready';
      return;
    }
    selectedSession = result.value.summary;
    selectedReport = result.value.report;
    if (selectedReport === null) {
      const reportResult = await invokeValidated(
        'get_report',
        { request: { session_id: sessionId } },
        commandResponseSchemas.get_report
      );
      if (reportResult.ok) selectedReport = reportResult.value.report;
    }
    reportStatus = 'ready';
    if (selectedSession?.kind === 'imported') {
      const reevaluation = await invokeValidated(
        'reevaluate_report',
        { request: { session_id: sessionId } },
        commandResponseSchemas.reevaluate_report
      );
      if (reevaluation.ok) reevaluatedReport = reevaluation.value.report;
      else reevaluationUnavailable = true;
    }
  }

  async function deleteSession(sessionId: string): Promise<void> {
    const result = await invokeValidated(
      'delete_session',
      {
        request: { session_id: sessionId, confirmation_token: 'user-confirmed' }
      },
      commandResponseSchemas.delete_session
    );
    if (result.ok) {
      if (selectedSession?.session_id === sessionId) {
        view = 'list';
        selectedSession = null;
        selectedReport = null;
      }
      await loadSessions();
    }
  }

  async function toggleReference(summary: SessionSummary): Promise<void> {
    const result = await invokeValidated(
      'set_session_reference',
      {
        request: {
          session_id: summary.session_id,
          is_reference: !summary.is_reference
        }
      },
      commandResponseSchemas.set_session_reference
    );
    if (result.ok) await loadSessions();
  }

  async function refreshExportPreview(): Promise<void> {
    if (selectedSession === null) {
      exportPreview = null;
      return;
    }
    const result = await invokeValidated(
      'preview_export',
      {
        request: {
          scope: { kind: exportScope, session_id: selectedSession.session_id },
          format: exportFormat,
          anonymize: exportAnonymize
        }
      },
      commandResponseSchemas.preview_export
    );
    exportPreview = result.ok ? result.value : null;
  }

  async function openExport(scope: 'session' | 'report'): Promise<void> {
    exportScope = scope;
    exportOpen = true;
    await refreshExportPreview();
  }

  async function runExport(): Promise<void> {
    if (selectedSession === null) return;
    exportBusy = true;
    const result = await invokeValidated(
      'export',
      {
        request: {
          scope: { kind: exportScope, session_id: selectedSession.session_id },
          format: exportFormat,
          anonymize: exportAnonymize
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

  async function importSession(): Promise<void> {
    importOpen = true;
    importStatus = 'reading';
    importDescription = undefined;
    importedSessionId = undefined;
    const result = await invokeValidated(
      'import_session',
      undefined,
      commandResponseSchemas.import_session
    );
    if (!result.ok) {
      importStatus = 'error';
      importDescription = t('sessions.errorDescription');
      return;
    }
    importStatus = 'success';
    importDescription = t('sessions.importSuccess');
    importedSessionId = result.value.session_id;
    await loadSessions();
  }

  onMount(() => {
    void loadSessions();
    const onOpenExport = (): void => {
      const candidate =
        selectedSession ??
        sessions.find((session) => session.status === 'active') ??
        sessions[0];
      if (candidate === undefined) {
        openExportWhenReady = true;
        return;
      }
      selectedSession = candidate;
      void openExport('session');
    };
    window.addEventListener('throttlewatch:open-export', onOpenExport);
    let stopSessionChanged: (() => void) | undefined;
    let stopReportFrozen: (() => void) | undefined;
    void listenValidated('session:changed', (value) => {
      const event = parseEvent('session:changed', value);
      if (event.ok) void loadSessions();
    }).then((stop) => (stopSessionChanged = stop));
    void listenValidated('report:frozen', (value) => {
      const event = parseEvent('report:frozen', value);
      if (event.ok && selectedSession?.session_id === event.value.session_id) {
        void openSession(event.value.session_id);
      }
    }).then((stop) => (stopReportFrozen = stop));
    let stopImportProgress: (() => void) | undefined;
    void listenValidated('import:progress', (value) => {
      const event = parseEvent('import:progress', value);
      if (event.ok) importStatus = event.value.phase;
    }).then((stop) => (stopImportProgress = stop));
    return () => {
      stopSessionChanged?.();
      stopReportFrozen?.();
      stopImportProgress?.();
      window.removeEventListener('throttlewatch:open-export', onOpenExport);
    };
  });
</script>

{#if view === 'list'}
  <div class="session-list-shell">
    <div class="session-actions">
      <Button
        variant="secondary"
        label={t('sessions.importSession')}
        onclick={() => void importSession()}
      />
    </div>
    <SessionsScreen
      status={listStatus}
      sessions={screenSessions}
      title={t('sessions.title')}
      onDeleteSession={(sessionId) => void deleteSession(sessionId)}
      loadingLabel={t('sessions.loading')}
      emptyTitle={t('sessions.emptyTitle')}
      emptyDescription={t('sessions.emptyDescription')}
      errorTitle={t('sessions.errorTitle')}
      errorDescription={t('sessions.errorDescription')}
      retryLabel={t('sessions.retry')}
      onRetry={() => void loadSessions()}
      deleteDialogTitle={t('sessions.deleteTitle')}
      deleteDialogDescription={t('sessions.deleteDescription')}
      deleteDialogCancelLabel={t('sessions.deleteCancel')}
      deleteDialogConfirmLabel={t('sessions.deleteConfirm')}
    />
  </div>
{:else}
  <div class="report-shell">
    <div class="report-actions">
      <Button
        variant="secondary"
        label={t('sessions.reportBack')}
        onclick={() => (view = 'list')}
      />
      <Button
        variant="secondary"
        label={t('sessions.exportSession')}
        disabled={selectedSession === null}
        onclick={() => void openExport('session')}
      />
      <Button
        variant="secondary"
        label={t('sessions.exportReport')}
        disabled={selectedSession === null}
        onclick={() => void openExport('report')}
      />
    </div>
    <ReportScreen
      status={reportStatus}
      loadingLabel={t('sessions.reportLoading')}
      classification={reportClassification}
      classificationLabel={reportClassificationLabel}
      headline={reportUnavailable
        ? t('sessions.reportUnavailable')
        : selectedReport === null
          ? t('sessions.reportNoReport')
          : t('sessions.reportHeadline')}
      sessionIncomplete={selectedSession?.status === 'incomplete' ||
        selectedSession?.status === 'cancelled'}
      incompleteNoticeTitle={t('sessions.reportIncomplete')}
      provisional={selectedSession?.status === 'active'}
      provisionalNoticeTitle={t('sessions.reportProvisional')}
      reducedConfidence={selectedSession?.coverage_tier !== 'A'}
      reducedConfidenceNoticeTitle={t('sessions.reportReducedConfidence')}
      observedTitle={t('sessions.reportObservedTitle')}
      observedText={t('sessions.reportObserved')}
      impactTitle={t('sessions.reportImpactTitle')}
      impactValue={reportImpact}
      impactUnavailableTitle={t('sessions.reportNoImpact')}
      impactUnavailableReason={t('sessions.reportNoImpactReason')}
      evidenceTitle={t('sessions.reportEvidenceTitle')}
      evidence={reportEvidence}
      alternativeCausesTitle={t('sessions.reportAlternativesTitle')}
      cannotConcludeTitle={t('sessions.reportCannotConcludeTitle')}
      recommendationsTitle={t('sessions.reportRecommendationsTitle')}
      methodTitle={t('sessions.reportMethodTitle')}
      methodDescription={selectedReport
        ? t('sessions.reportMethod')
        : undefined}
      methodMissingText={t('sessions.reportMethodMissing')}
      comparisonTitle={t('sessions.reportComparisonTitle')}
      reevaluated={reevaluatedView}
    />
  </div>
{/if}

<ExportDialog
  bind:open={exportOpen}
  title={exportScope === 'report'
    ? t('sessions.exportReport')
    : t('sessions.exportSession')}
  scopeLabel={selectedSession?.started_at ?? t('sessions.title')}
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
  anonymize={exportAnonymize}
  onAnonymizeChange={(value) => {
    exportAnonymize = value;
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
  confirmLabel={exportBusy ? t('sessions.loading') : t('analysis.save')}
  confirmDisabled={exportPreview === null || exportBusy}
  onCancel={cancelExport}
  onConfirm={() => void runExport()}
/>

<ImportResultDialog
  bind:open={importOpen}
  title={t('sessions.importTitle')}
  status={importStatus}
  progressLabel={importStatus === 'reading'
    ? t('sessions.importReading')
    : importStatus === 'validating'
      ? t('sessions.importValidating')
      : importStatus === 'migrating'
        ? t('sessions.importMigrating')
        : t('sessions.importStoring')}
  description={importDescription}
  closeLabel={t('sessions.importClose')}
  onClose={() => (importOpen = false)}
  openSessionLabel={t('sessions.importOpen')}
  onOpenSession={() => {
    if (importedSessionId) {
      importOpen = false;
      void openSession(importedSessionId);
    }
  }}
/>

<style>
  .report-shell {
    max-width: 860px;
    margin: 0 auto;
    padding: var(--space-6);
  }
  .session-list-shell {
    min-height: 100%;
  }
  .session-actions,
  .report-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    max-width: 1100px;
    margin: 0 auto;
    padding: var(--space-4) var(--space-6) 0;
  }
  .report-actions {
    padding-top: var(--space-6);
  }
</style>
