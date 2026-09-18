<script lang="ts">
  /**
   * The "Análisis" screen: `AnalysisChart` plus the evidence panel that
   * explains whatever the person has pointed at. Presentational, like
   * every other screen in this system — `status`, `tracks`, `events`
   * and the evidence objects are entirely host-computed; this
   * component does not look up an event by id, compute a range's
   * statistics, or decide what counts as stale/partial data. It only
   * renders what it's given and reports intent outward
   * (`onSelectEvent`, `onRangeSelect`).
   *
   * Evidence priority (the frontend spec's own precedence, not a
   * judgment this component makes): `selectedEventEvidence` first (a
   * click on an event band), then `rangeEvidence` (a completed brush
   * selection with no event selected), then the idle hint. The host is
   * expected to clear `selectedEventEvidence` itself when the person
   * deselects (via `onSelectEvent(undefined)`) — this component never
   * clears props it doesn't own.
   *
   * `dataNoticeTitle` is this screen's one banner slot for "datos
   * obsoletos" / "datos parciales" — a single optional notice, not a
   * queue, matching `SettingsScreen`'s single risk-zone banner rather
   * than `GuidedDiagnosticScreen`'s phase-specific ones (Análisis has
   * exactly one kind of standing notice to show at a time).
   *
   * `valueLabel`/`cursorLabel`/`rangeSummaryLabel`/`resetRangeLabel`
   * are passed straight through to `AnalysisChart` — see that
   * component's own doc comment for what each one drives (there is no
   * copy of any kind left inside either component; every visible or
   * accessible string is one of these callbacks/props).
   */
  import type { Snippet } from 'svelte';
  import AnalysisChart, {
    type AnalysisTrack,
    type AnalysisEvent,
    type AnalysisPoint
  } from './AnalysisChart.svelte';
  import EmptyState from './EmptyState.svelte';
  import Banner from './Banner.svelte';
  import ProgressBar from './ProgressBar.svelte';

  export type AnalysisStatus = 'noHistory' | 'loading' | 'ready';

  export interface AnalysisEvidenceItem {
    label: string;
    value: string;
  }

  export interface AnalysisEvidence {
    title: string;
    description?: string;
    items?: AnalysisEvidenceItem[];
  }

  interface Props {
    status: AnalysisStatus;
    loadingLabel: string;
    noHistoryTitle: string;
    noHistoryDescription?: string;
    noHistoryIcon?: Snippet;
    dataNoticeTone?: 'warning' | 'critical';
    dataNoticeTitle?: string;
    dataNoticeDescription?: string;
    tracks: AnalysisTrack[];
    events?: AnalysisEvent[];
    selectedEventId?: string;
    onSelectEvent?: (id: string | undefined) => void;
    valueLabel: (point: AnalysisPoint | undefined, track: AnalysisTrack) => string;
    timeLabel: (t: number) => string;
    legendLabel: string;
    cursorLabel: string;
    rangeSummaryLabel: (range: [number, number]) => string;
    resetRangeLabel: string;
    onRangeSelect?: (range: [number, number] | null) => void;
    selectedEventEvidence?: AnalysisEvidence;
    rangeEvidence?: AnalysisEvidence;
    idleHint: string;
    evidenceTitle: string;
  }

  let {
    status,
    loadingLabel,
    noHistoryTitle,
    noHistoryDescription,
    noHistoryIcon,
    dataNoticeTone = 'warning',
    dataNoticeTitle,
    dataNoticeDescription,
    tracks,
    events = [],
    selectedEventId,
    onSelectEvent,
    valueLabel,
    timeLabel,
    legendLabel,
    cursorLabel,
    rangeSummaryLabel,
    resetRangeLabel,
    onRangeSelect,
    selectedEventEvidence,
    rangeEvidence,
    idleHint,
    evidenceTitle
  }: Props = $props();

  let activeEvidence = $derived(selectedEventEvidence ?? rangeEvidence ?? null);
</script>

<div class="tw-ds tw-analysis-screen">
  {#if status === 'loading'}
    <div class="status-block">
      <ProgressBar indeterminate tone="accent" label={loadingLabel} />
      <span class="caption" style:color="var(--text-tertiary)">{loadingLabel}</span>
    </div>
  {:else if status === 'noHistory'}
    <EmptyState icon={noHistoryIcon} title={noHistoryTitle} description={noHistoryDescription} />
  {:else}
    {#if dataNoticeTitle}
      <Banner tone={dataNoticeTone} title={dataNoticeTitle} description={dataNoticeDescription} />
    {/if}

    <div class="layout">
      <div class="chart-col">
        <AnalysisChart
          {tracks}
          {events}
          {selectedEventId}
          {onSelectEvent}
          {valueLabel}
          {timeLabel}
          {legendLabel}
          {cursorLabel}
          {rangeSummaryLabel}
          {resetRangeLabel}
          {onRangeSelect}
        />
      </div>

      <aside class="evidence-col" aria-label={evidenceTitle}>
        <span class="caption evidence-heading" style:color="var(--text-tertiary)">{evidenceTitle}</span>
        {#if activeEvidence}
          <div class="evidence-card">
            <span class="body-strong" style:color="var(--text-primary)">{activeEvidence.title}</span>
            {#if activeEvidence.description}
              <span class="body" style:color="var(--text-secondary)">{activeEvidence.description}</span>
            {/if}
            {#if activeEvidence.items && activeEvidence.items.length > 0}
              <dl class="evidence-items">
                {#each activeEvidence.items as item (item.label)}
                  <div class="evidence-item">
                    <dt class="caption" style:color="var(--text-tertiary)">{item.label}</dt>
                    <dd class="body-strong" style:color="var(--text-primary)">{item.value}</dd>
                  </div>
                {/each}
              </dl>
            {/if}
          </div>
        {:else}
          <span class="body idle-hint" style:color="var(--text-tertiary)">{idleHint}</span>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .tw-analysis-screen {
    background: transparent;
    padding: var(--space-6);
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    container-type: inline-size;
    container-name: tw-analysis;
  }
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-6) 0;
  }
  .layout {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
  }
  .chart-col {
    flex: 1;
    min-width: 0;
  }
  .evidence-col {
    flex: 0 0 260px;
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
    padding: var(--space-4);
    position: sticky;
    top: var(--space-4);
  }
  .evidence-heading {
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .evidence-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .evidence-items {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
  }
  .evidence-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .evidence-item dd {
    margin: 0;
  }
  .idle-hint {
    max-width: 34ch;
  }

  @container tw-analysis (max-width: 699px) {
    .layout {
      flex-direction: column;
    }
    .evidence-col {
      flex: 1 1 auto;
      position: static;
      width: 100%;
    }
  }
</style>
