<script lang="ts">
  /**
   * The one export dialog (FR-026/FR-028, HU-07), reused for the three
   * scopes the product exports — a whole session, a report, or a
   * selected range of "Análisis". The scope is described by
   * `scopeLabel`; the dialog itself does not know or care which one.
   *
   * Shows: format choice (CSV samples / JSON report), the anonymize
   * switch (pre-set from the `privacy.anonymize_exports` preference
   * by the host), and two lists — fields included and fields excluded
   * — that the host recomputes whenever format/anonymize change
   * (`preview_export` in application-commands.md). Also the estimated
   * size and proposed file name. The destination folder is never shown
   * nor collected here: the native save dialog owns that.
   *
   * Presentational: `format`/`anonymize` are controlled props with
   * change callbacks; the lists are host-supplied. No copy lives here.
   */
  import Dialog from './Dialog.svelte';
  import Button from './Button.svelte';
  import SegmentedControl from './SegmentedControl.svelte';
  import Switch from './Switch.svelte';

  export type ExportFormat = 'csv' | 'json';

  interface Props {
    open?: boolean;
    title: string;
    /** e.g. "Sesión del 16 sept 2026 · 14 min" or "Rango seleccionado · 02:10–05:40". */
    scopeLabel: string;
    formatLabel: string;
    format: ExportFormat;
    formatOptions: { value: ExportFormat; label: string }[];
    onFormatChange: (format: ExportFormat) => void;
    anonymizeLabel: string;
    anonymizeDescription?: string;
    anonymize: boolean;
    onAnonymizeChange: (checked: boolean) => void;
    includedTitle: string;
    includedFields: string[];
    excludedTitle: string;
    excludedFields: string[];
    /** e.g. "Tamaño estimado: 1,8 MB" */
    sizeLabel?: string;
    /** e.g. "throttlewatch_session_2026-09-16_a1b2c3.csv" */
    fileNameLabel?: string;
    cancelLabel: string;
    confirmLabel: string;
    confirmDisabled?: boolean;
    onCancel: () => void;
    onConfirm: () => void;
  }

  let {
    open = $bindable(false),
    title,
    scopeLabel,
    formatLabel,
    format,
    formatOptions,
    onFormatChange,
    anonymizeLabel,
    anonymizeDescription,
    anonymize,
    onAnonymizeChange,
    includedTitle,
    includedFields,
    excludedTitle,
    excludedFields,
    sizeLabel,
    fileNameLabel,
    cancelLabel,
    confirmLabel,
    confirmDisabled = false,
    onCancel,
    onConfirm
  }: Props = $props();

  // `Dialog.onclose` also fires after a programmatic close (the native
  // <dialog> `close` event does not distinguish), so an explicit choice
  // must not be followed by the dismiss callback. `decided` is reset
  // each time the dialog opens.
  let decided = $state(false);
  $effect(() => {
    if (open) decided = false;
  });

  function confirm() {
    decided = true;
    onConfirm();
  }
  function handleClose() {
    if (!decided) onCancel();
  }
</script>

<Dialog bind:open {title} size="wide" onclose={handleClose}>
  {#snippet body()}
    <div class="tw-export-body">
      <span class="body scope" style:color="var(--text-secondary)">{scopeLabel}</span>

      <div class="row">
        <span class="body-strong" style:color="var(--text-primary)">{formatLabel}</span>
        <SegmentedControl
          label={formatLabel}
          value={format}
          options={formatOptions}
          onchange={(v) => onFormatChange(v as ExportFormat)}
        />
      </div>

      <div class="row">
        <div class="row-text">
          <span class="body-strong" style:color="var(--text-primary)">{anonymizeLabel}</span>
          {#if anonymizeDescription}
            <span class="caption" style:color="var(--text-secondary)">{anonymizeDescription}</span>
          {/if}
        </div>
        <Switch checked={anonymize} label={anonymizeLabel} onchange={onAnonymizeChange} />
      </div>

      <div class="fields">
        <div class="field-col">
          <span class="caption col-title" style:color="var(--text-tertiary)">{includedTitle}</span>
          <ul class="field-list">
            {#each includedFields as field (field)}
              <li class="body included">
                <span class="glyph" aria-hidden="true"><svg viewBox="0 0 16 16" width="12" height="12"><path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg></span>
                {field}
              </li>
            {/each}
          </ul>
        </div>
        <div class="field-col">
          <span class="caption col-title" style:color="var(--text-tertiary)">{excludedTitle}</span>
          {#if excludedFields.length === 0}
            <span class="body" style:color="var(--text-tertiary)" aria-hidden="true">—</span>
          {:else}
            <ul class="field-list">
              {#each excludedFields as field (field)}
                <li class="body excluded">
                  <span class="glyph" aria-hidden="true"><svg viewBox="0 0 16 16" width="12" height="12"><path d="M4 8 L12 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg></span>
                  {field}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>

      {#if sizeLabel || fileNameLabel}
        <div class="meta">
          {#if fileNameLabel}
            <span class="caption mono" style:color="var(--text-secondary)">{fileNameLabel}</span>
          {/if}
          {#if sizeLabel}
            <span class="caption" style:color="var(--text-tertiary)">{sizeLabel}</span>
          {/if}
        </div>
      {/if}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button variant="secondary" label={cancelLabel} onclick={onCancel} />
    <Button variant="primary" label={confirmLabel} disabled={confirmDisabled} onclick={confirm} />
  {/snippet}
</Dialog>

<style>
  .tw-export-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
  }
  .field-col {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }
  .col-title {
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .field-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field-list li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
  }
  .field-list li.included .glyph {
    color: var(--status-normal);
  }
  .field-list li.excluded .glyph {
    color: var(--text-tertiary);
  }
  .field-list li.excluded {
    text-decoration: line-through;
    text-decoration-color: var(--text-tertiary);
  }
  .glyph {
    display: flex;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .mono {
    font-variant-numeric: tabular-nums;
    word-break: break-all;
  }
  @media (max-width: 479px) {
    .fields {
      grid-template-columns: 1fr;
    }
    .row {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
