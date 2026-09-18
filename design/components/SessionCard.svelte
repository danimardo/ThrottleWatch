<script lang="ts">
  /**
   * One row of the Sesiones list: fecha, duración, tipo, and either a
   * `StatusChip` (terminada/importada with a known classification) or
   * a plain muted status tag (activa/cancelada/incompleta — states
   * that don't carry a diagnostic classification yet or at all).
   *
   * Presentational only — `status` picks which tag/actions render,
   * nothing here decides *when* a session becomes 'active' or
   * 'incomplete'. `exportDisabled`/`exportDisabledReason` follow the
   * same "never hide, explain instead" rule as `OptionRow`.
   *
   * Deleting a session needs its own confirmation — this card does
   * NOT show a confirm `Dialog` itself (that would mean one `<dialog>`
   * per row). It calls `onRequestDelete`, and the host (normally
   * `SessionsScreen`, which owns one shared confirm dialog) decides
   * what that means. `onOpen`/`onExport` fire directly: neither is
   * destructive.
   */
  import StatusChip from './StatusChip.svelte';
  import type { Classification } from '../lib/classification';

  export type SessionStatus = 'active' | 'completed' | 'cancelled' | 'incomplete' | 'imported';

  interface Props {
    status: SessionStatus;
    typeLabel: string;
    dateLabel: string;
    durationLabel?: string;
    /** Shown as a muted tag when there's no classification chip to show (active/cancelled/incomplete), e.g. "En curso", "Cancelada", "Incompleta". */
    statusLabel?: string;
    /** Only for 'completed' / 'imported' sessions that have a result. */
    classification?: Classification;
    classificationLabel?: string;
    /** 'imported' sessions get a small secondary tag next to the classification chip (or alone, if there's no classification yet). */
    importedLabel?: string;
    /** True when this session backs a `user_marked` baseline; renders the `referenceLabel` tag. */
    isReference?: boolean;
    referenceLabel?: string;
    /** "Usar como referencia" / "Retirar referencia" — the host picks the wording from `isReference`. Only meaningful for `completed`. */
    referenceActionLabel?: string;
    onToggleReference?: () => void;
    selected?: boolean;
    openLabel: string;
    onOpen: () => void;
    exportLabel?: string;
    onExport?: () => void;
    exportDisabled?: boolean;
    exportDisabledReason?: string;
    deleteLabel?: string;
    onRequestDelete?: () => void;
    /** Position in the list, for the staggered entrance (`SessionsScreen` passes it). */
    enterIndex?: number;
  }

  let {
    status,
    typeLabel,
    dateLabel,
    durationLabel,
    statusLabel,
    classification,
    classificationLabel,
    importedLabel,
    isReference = false,
    referenceLabel,
    referenceActionLabel,
    onToggleReference,
    selected = false,
    openLabel,
    onOpen,
    exportLabel,
    onExport,
    exportDisabled = false,
    exportDisabledReason,
    deleteLabel,
    onRequestDelete,
    enterIndex
  }: Props = $props();
</script>

<div class="tw-session-card" class:selected class:tw-enter={enterIndex !== undefined} style:--tw-i={enterIndex} role="listitem">
  <div class="main">
    <div class="heading">
      {#if status === 'active'}
        <span class="live-dot" aria-hidden="true"></span>
      {/if}
      <span class="body-strong" style:color="var(--text-primary)">{typeLabel}</span>
    </div>
    <div class="meta">
      <span class="caption" style:color="var(--text-tertiary)">{dateLabel}</span>
      {#if durationLabel}
        <span class="caption dot" style:color="var(--text-tertiary)" aria-hidden="true">·</span>
        <span class="caption" style:color="var(--text-tertiary)">{durationLabel}</span>
      {/if}
    </div>
  </div>

  <div class="result">
    {#if classification && classificationLabel}
      <StatusChip {classification} label={classificationLabel} />
    {:else if statusLabel}
      <span class="status-tag caption">{statusLabel}</span>
    {/if}
    {#if status === 'imported' && importedLabel}
      <span class="status-tag caption imported">{importedLabel}</span>
    {/if}
    {#if isReference && referenceLabel}
      <span class="status-tag caption reference">{referenceLabel}</span>
    {/if}
  </div>

  <div class="actions">
    <button type="button" class="btn-open label" onclick={onOpen}>{openLabel}</button>
    {#if onToggleReference && referenceActionLabel && status === 'completed'}
      <button type="button" class="icon-btn" class:reference-on={isReference} aria-label={referenceActionLabel} aria-pressed={isReference} onclick={onToggleReference}>
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path
            d="M8 2.2 L9.8 6 L14 6.5 L10.9 9.4 L11.7 13.6 L8 11.6 L4.3 13.6 L5.1 9.4 L2 6.5 L6.2 6 Z"
            fill={isReference ? 'currentColor' : 'none'}
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
    {#if onExport}
      <button
        type="button"
        class="icon-btn"
        aria-label={exportLabel}
        title={exportDisabled ? exportDisabledReason : undefined}
        disabled={exportDisabled}
        onclick={onExport}
      >
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path
            d="M8 2 L8 9.5 M5 6.5 L8 9.5 L11 6.5 M3 12 L3 13.2 A0.8 0.8 0 0 0 3.8 14 L12.2 14 A0.8 0.8 0 0 0 13 13.2 L13 12"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
    {#if onRequestDelete}
      <button type="button" class="icon-btn danger" aria-label={deleteLabel} onclick={onRequestDelete}>
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path
            d="M3.5 4.5 L4.2 13 A1 1 0 0 0 5.2 14 L10.8 14 A1 1 0 0 0 11.8 13 L12.5 4.5 M2.5 4.5 L13.5 4.5 M6.2 4.5 L6.5 2.6 A0.8 0.8 0 0 1 7.3 2 L8.7 2 A0.8 0.8 0 0 1 9.5 2.6 L9.8 4.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    {/if}
  </div>
</div>

<style>
  .tw-session-card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    transition:
      transform var(--motion-base) var(--motion-spring),
      box-shadow var(--motion-base) var(--motion-ease-out);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-session-card:hover {
    transform: translateY(-1px);
    box-shadow: var(--glass-shadow-strong);
  }
  .tw-session-card.selected {
    border-color: var(--accent-blue);
    background: color-mix(in srgb, var(--accent-blue) 6%, var(--surface));
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .heading {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .live-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent-blue);
    flex: 0 0 auto;
    animation: tw-session-pulse 1.6s ease-in-out infinite;
  }
  @keyframes tw-session-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .result {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .status-tag {
    display: inline-flex;
    align-items: center;
    padding: 5px 11px;
    border-radius: var(--radius-full);
    background: var(--surface-sunken);
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .status-tag.reference {
    color: var(--accent-blue);
    border-color: color-mix(in srgb, var(--accent-blue) 40%, var(--hairline));
  }
  .icon-btn.reference-on {
    color: var(--accent-blue);
  }
  .status-tag.imported {
    background: transparent;
    border: 1px solid var(--hairline);
    color: var(--text-tertiary);
  }
  .actions {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .btn-open {
    border: 1px solid var(--hairline);
    background: var(--surface-raised);
    color: var(--text-primary);
    padding: 7px 14px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }
  .btn-open:hover {
    background: var(--surface-sunken);
  }
  .icon-btn {
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    padding: 7px;
    border-radius: var(--radius-sm);
    display: flex;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-sunken);
    color: var(--text-primary);
  }
  .icon-btn.danger:hover:not(:disabled) {
    color: var(--status-thermal);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .icon-btn:focus-visible,
  .btn-open:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }

  @container tw-sessions (max-width: 599px) {
    .tw-session-card {
      flex-wrap: wrap;
    }
    .result {
      order: 3;
      flex: 1 0 100%;
    }
    .actions {
      order: 2;
    }
  }
</style>
