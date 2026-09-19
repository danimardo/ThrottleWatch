<script lang="ts">
  /**
   * The "Sesiones" screen: a loading/error/empty/loaded list of
   * `SessionCard`s. Presentational — `status` and the `sessions` array
   * are entirely host-computed; this component never fetches, sorts,
   * or filters sessions itself. Its only local state is which
   * session's delete confirmation dialog is open (one shared `Dialog`,
   * not one per card, same shape as `SettingsScreen`'s two confirm
   * dialogs).
   */
  import type { Snippet } from 'svelte';
  import SessionCard, { type SessionStatus } from './SessionCard.svelte';
  import type { Classification } from '../lib/classification';
  import EmptyState from './EmptyState.svelte';
  import Banner from './Banner.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Button from './Button.svelte';
  import Dialog from './Dialog.svelte';

  export type SessionsListStatus = 'loading' | 'error' | 'loaded';

  export interface SessionsScreenSession {
    id: string;
    status: SessionStatus;
    typeLabel: string;
    dateLabel: string;
    durationLabel?: string;
    statusLabel?: string;
    classification?: Classification;
    classificationLabel?: string;
    importedLabel?: string;
    isReference?: boolean;
    referenceLabel?: string;
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
  }

  interface Props {
    status: SessionsListStatus;
    sessions: SessionsScreenSession[];
    /** Optional header: a title and the "Importar…" action (HU-07). */
    title?: string;
    importLabel?: string;
    onImport?: () => void;
    importDisabled?: boolean;
    onDeleteSession: (id: string) => void;
    loadingLabel: string;
    emptyTitle: string;
    emptyDescription?: string;
    emptyIcon?: Snippet;
    emptyAction?: Snippet;
    errorTitle: string;
    errorDescription?: string;
    retryLabel: string;
    onRetry: () => void;
    deleteDialogTitle: string;
    deleteDialogDescription: string;
    deleteDialogCancelLabel: string;
    deleteDialogConfirmLabel: string;
  }

  let {
    status,
    sessions,
    title,
    importLabel,
    onImport,
    importDisabled = false,
    onDeleteSession,
    loadingLabel,
    emptyTitle,
    emptyDescription,
    emptyIcon,
    emptyAction,
    errorTitle,
    errorDescription,
    retryLabel,
    onRetry,
    deleteDialogTitle,
    deleteDialogDescription,
    deleteDialogCancelLabel,
    deleteDialogConfirmLabel
  }: Props = $props();

  let pendingDeleteId: string | null = $state(null);
  let deleteDialogOpen = $derived(pendingDeleteId !== null);

  function requestDelete(id: string) {
    pendingDeleteId = id;
  }
  function cancelDelete() {
    pendingDeleteId = null;
  }
  function confirmDelete() {
    if (pendingDeleteId !== null) onDeleteSession(pendingDeleteId);
    pendingDeleteId = null;
  }
</script>

{#snippet retryAction()}
  <Button variant="secondary" label={retryLabel} onclick={onRetry} />
{/snippet}

<div class="tw-ds tw-sessions-screen">
  {#if title || (onImport && importLabel)}
    <div class="screen-head">
      {#if title}
        <h2 class="value-md" style:color="var(--text-primary)">{title}</h2>
      {/if}
      {#if onImport && importLabel}
        <Button variant="secondary" label={importLabel} disabled={importDisabled} onclick={onImport} />
      {/if}
    </div>
  {/if}
  {#if status === 'loading'}
    <div class="status-block">
      <ProgressBar indeterminate tone="accent" label={loadingLabel} />
      <span class="caption" style:color="var(--text-tertiary)">{loadingLabel}</span>
    </div>
  {:else if status === 'error'}
    <Banner tone="critical" title={errorTitle} description={errorDescription} action={retryAction} />
  {:else if sessions.length === 0}
    <EmptyState icon={emptyIcon} title={emptyTitle} description={emptyDescription} action={emptyAction} />
  {:else}
    <div class="list" role="list">
      {#each sessions as session, i (session.id)}
        <SessionCard
          enterIndex={i}
          status={session.status}
          typeLabel={session.typeLabel}
          dateLabel={session.dateLabel}
          durationLabel={session.durationLabel}
          statusLabel={session.statusLabel}
          classification={session.classification}
          classificationLabel={session.classificationLabel}
          importedLabel={session.importedLabel}
          isReference={session.isReference}
          referenceLabel={session.referenceLabel}
          referenceActionLabel={session.referenceActionLabel}
          onToggleReference={session.onToggleReference}
          selected={session.selected}
          openLabel={session.openLabel}
          onOpen={session.onOpen}
          exportLabel={session.exportLabel}
          onExport={session.onExport}
          exportDisabled={session.exportDisabled}
          exportDisabledReason={session.exportDisabledReason}
          deleteLabel={session.deleteLabel}
          onRequestDelete={session.deleteLabel ? () => requestDelete(session.id) : undefined}
        />
      {/each}
    </div>
  {/if}
</div>

<Dialog
  open={deleteDialogOpen}
  title={deleteDialogTitle}
  description={deleteDialogDescription}
  tone="warning"
  onclose={cancelDelete}
>
  {#snippet actions()}
    <Button variant="secondary" label={deleteDialogCancelLabel} onclick={cancelDelete} />
    <Button variant="destructive" label={deleteDialogConfirmLabel} onclick={confirmDelete} />
  {/snippet}
</Dialog>

<style>
  .screen-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .screen-head h2 {
    margin: 0;
  }
  .tw-sessions-screen {
    background: transparent;
    padding: var(--space-6);
    max-width: 860px;
    margin: 0 auto;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    container-type: inline-size;
    container-name: tw-sessions;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-6) 0;
  }
</style>
