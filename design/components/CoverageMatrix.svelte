<script lang="ts">
  /**
   * The "Cobertura" panel (FR-023): one row per magnitude the
   * diagnostic engine cares about, saying whether it is measured on
   * THIS machine, with what quality, from which sensor, and — when it
   * is missing — why. The footer states the strongest conclusion the
   * machine allows ("confianza máxima alcanzable") and the state of
   * the low-level access, with the install/repair action rendered
   * only for `installable` (see lib/access.ts).
   *
   * Reused in two places: as the panel `ContextStrip`'s "Ver cobertura"
   * opens on "Ahora", and embedded in Ajustes → Sensores y cobertura.
   *
   * Presentational: rows, labels, quality wording and the confidence
   * sentence are all host-computed. The component never infers a
   * quality or a ceiling from the rows. No copy lives here (rule 2).
   *
   * "Available" is communicated with a glyph AND text, never color
   * alone; `substitute` quality additionally gets a dashed underline
   * so a reduced-quality reading reads as a pattern, matching
   * AnalysisChart's dashed segments and CoreCell's hatch.
   */
  import type { AdvancedAccessState } from '../lib/access';
  import Button from './Button.svelte';
  import Banner from './Banner.svelte';

  export type CoverageQuality = 'direct' | 'derived' | 'substitute';

  export interface CoverageRow {
    id: string;
    /** e.g. "Temperatura", "Reloj efectivo" */
    label: string;
    available: boolean;
    quality?: CoverageQuality;
    /** Already-translated quality word, e.g. "Directo" / "Derivado" / "Sustituto". */
    qualityLabel?: string;
    /** Original sensor name or derivation, e.g. "CPU Package" / "% Processor Performance". */
    sourceLabel?: string;
    /** Why it is missing, when `available` is false. */
    reasonLabel?: string;
  }

  export interface CoverageColumnLabels {
    magnitude: string;
    available: string;
    quality: string;
    source: string;
    reason: string;
  }

  interface Props {
    title?: string;
    rows: CoverageRow[];
    columnLabels: CoverageColumnLabels;
    availableLabel: string;
    unavailableLabel: string;
    /** Full sentence, e.g. "Confianza máxima alcanzable en este equipo: probable". */
    maxConfidenceLabel: string;
    accessState: AdvancedAccessState;
    /** Sentence describing `accessState` to the person, host-translated. */
    accessLabel: string;
    requestAccessLabel?: string;
    onRequestAccess?: () => void;
    accessRetryLabel?: string;
    onAccessRetry?: () => void;
    recheckLabel: string;
    onRecheck: () => void;
    recheckDisabled?: boolean;
    copySummaryLabel?: string;
    onCopySummary?: () => void;
  }

  let {
    title,
    rows,
    columnLabels,
    availableLabel,
    unavailableLabel,
    maxConfidenceLabel,
    accessState,
    accessLabel,
    requestAccessLabel,
    onRequestAccess,
    accessRetryLabel,
    onAccessRetry,
    recheckLabel,
    onRecheck,
    recheckDisabled = false,
    copySummaryLabel,
    onCopySummary
  }: Props = $props();

  let accessTone = $derived<'info' | 'warning' | 'critical'>(
    accessState === 'installable' ? 'info' : accessState === 'denied' ? 'warning' : accessState === 'error' ? 'critical' : 'info'
  );
  let showAccessBanner = $derived(accessState === 'installable' || accessState === 'denied' || accessState === 'error');
</script>

{#snippet requestAction()}
  {#if onRequestAccess}
    <Button variant="secondary" label={requestAccessLabel ?? ''} onclick={onRequestAccess} />
  {/if}
{/snippet}
{#snippet retryAction()}
  {#if onAccessRetry}
    <Button variant="secondary" label={accessRetryLabel ?? ''} onclick={onAccessRetry} />
  {/if}
{/snippet}

<div class="tw-ds tw-coverage-matrix">
  {#if title}
    <h3 class="label title">{title}</h3>
  {/if}

  <table class="matrix">
    <thead>
      <tr>
        <th class="caption" scope="col">{columnLabels.magnitude}</th>
        <th class="caption" scope="col">{columnLabels.available}</th>
        <th class="caption" scope="col">{columnLabels.quality}</th>
        <th class="caption" scope="col">{columnLabels.source}</th>
        <th class="caption" scope="col">{columnLabels.reason}</th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i (row.id)}
        <tr class:unavailable={!row.available} style:--tw-i={i}>
          <th scope="row" class="body-strong">{row.label}</th>
          <td data-label={columnLabels.available}>
            <span class="avail" class:ok={row.available}>
              <span class="glyph" aria-hidden="true">
                {#if row.available}
                  <svg viewBox="0 0 16 16" width="14" height="14"><path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
                {:else}
                  <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 8 L12 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
                {/if}
              </span>
              <span class="body">{row.available ? availableLabel : unavailableLabel}</span>
            </span>
          </td>
          <td data-label={columnLabels.quality}>
            {#if row.available && row.qualityLabel}
              <span class="body quality" class:substitute={row.quality === 'substitute'} class:derived={row.quality === 'derived'}>{row.qualityLabel}</span>
            {:else}
              <span class="body muted" aria-hidden="true">—</span>
            {/if}
          </td>
          <td data-label={columnLabels.source}>
            <span class="body source">{row.sourceLabel ?? '—'}</span>
          </td>
          <td data-label={columnLabels.reason}>
            <span class="body muted">{row.available ? '' : (row.reasonLabel ?? '')}</span>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="footer">
    <span class="body-strong confidence">{maxConfidenceLabel}</span>
    {#if showAccessBanner}
      <Banner
        tone={accessTone}
        title={accessLabel}
        action={accessState === 'installable' && onRequestAccess ? requestAction : accessState === 'error' && onAccessRetry ? retryAction : undefined}
      />
    {:else}
      <span class="body access-line">{accessLabel}</span>
    {/if}
    <div class="actions">
      <Button variant="secondary" label={recheckLabel} disabled={recheckDisabled} onclick={onRecheck} />
      {#if onCopySummary && copySummaryLabel}
        <Button variant="secondary" label={copySummaryLabel} onclick={onCopySummary} />
      {/if}
    </div>
  </div>
</div>

<style>
  .tw-coverage-matrix {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    color: var(--text-primary);
    container-type: inline-size;
    container-name: tw-coverage;
  }
  .title {
    margin: 0;
    color: var(--text-tertiary);
  }
  .matrix {
    width: 100%;
    border-collapse: collapse;
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  tbody tr {
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    animation-delay: calc(var(--tw-i, 0) * var(--motion-stagger));
  }
  thead th {
    text-align: left;
    color: var(--text-tertiary);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--hairline);
    font-weight: 500;
  }
  tbody th,
  tbody td {
    text-align: left;
    padding: var(--space-3);
    border-bottom: 1px solid var(--hairline);
    vertical-align: top;
  }
  tbody tr:last-child th,
  tbody tr:last-child td {
    border-bottom: none;
  }
  tbody th {
    color: var(--text-primary);
    font-weight: 600;
  }
  tr.unavailable th {
    color: var(--text-secondary);
  }
  .avail {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-tertiary);
  }
  .avail.ok {
    color: var(--status-normal);
  }
  .avail .body {
    color: var(--text-secondary);
  }
  .glyph {
    display: flex;
  }
  .quality {
    color: var(--text-secondary);
  }
  .quality.derived {
    color: var(--text-secondary);
    font-style: italic;
  }
  .quality.substitute {
    color: var(--status-warm);
    text-decoration: underline dashed;
    text-underline-offset: 3px;
  }
  .source {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--text-tertiary);
  }
  .footer {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .confidence {
    color: var(--text-primary);
  }
  .access-line {
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  /* Compact: collapse the table into stacked cards, one per magnitude. */
  @container tw-coverage (max-width: 479px) {
    thead {
      display: none;
    }
    .matrix,
    tbody,
    tr,
    th,
    td {
      display: block;
    }
    tr {
      padding: var(--space-3);
      border-bottom: 1px solid var(--hairline);
    }
    tr:last-child {
      border-bottom: none;
    }
    tbody th,
    tbody td {
      padding: 2px 0;
      border-bottom: none;
    }
    td[data-label]::before {
      content: attr(data-label) ': ';
      color: var(--text-tertiary);
    }
    td:has(> .muted:empty) {
      display: none;
    }
  }
</style>
