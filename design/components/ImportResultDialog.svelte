<script lang="ts">
  /**
   * Shown after the person picks a file in the native open dialog
   * (`import_session` in application-commands.md): validation and
   * migration progress, then success (with the option to open the
   * imported session) or a failure with an actionable reason
   * ("versión más reciente que esta aplicación", "fichero dañado").
   * Warnings (e.g. "la referencia del fichero no se usará en este
   * equipo") are listed, never hidden.
   *
   * Presentational: `status` and every string are host-supplied.
   */
  import Dialog from './Dialog.svelte';
  import Button from './Button.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Banner from './Banner.svelte';

  export type ImportStatus = 'reading' | 'validating' | 'migrating' | 'success' | 'error';

  interface Props {
    open?: boolean;
    title: string;
    status: ImportStatus;
    /** Label for the in-progress states (reading/validating/migrating), host picks the wording per status. */
    progressLabel?: string;
    /** Sentence for success ("Sesión importada: 16 sept 2026, 14 min") or error (reason). */
    description?: string;
    warningsTitle?: string;
    warnings?: string[];
    closeLabel: string;
    onClose: () => void;
    openSessionLabel?: string;
    onOpenSession?: () => void;
  }

  let {
    open = $bindable(false),
    title,
    status,
    progressLabel,
    description,
    warningsTitle,
    warnings = [],
    closeLabel,
    onClose,
    openSessionLabel,
    onOpenSession
  }: Props = $props();

  let inProgress = $derived(status === 'reading' || status === 'validating' || status === 'migrating');

  // `Dialog.onclose` also fires after a programmatic close (the native
  // <dialog> `close` event does not distinguish), so an explicit choice
  // must not be followed by the dismiss callback. `decided` is reset
  // each time the dialog opens.
  let decided = $state(false);
  $effect(() => {
    if (open) decided = false;
  });

  function openSession() {
    decided = true;
    onOpenSession?.();
  }
  function handleClose() {
    if (!decided) onClose();
  }
</script>

<Dialog bind:open {title} tone={status === 'error' ? 'warning' : 'default'} onclose={handleClose}>
  {#snippet body()}
    <div class="tw-import-body">
      {#if inProgress}
        <ProgressBar indeterminate tone="accent" label={progressLabel} />
        {#if progressLabel}
          <span class="caption" style:color="var(--text-tertiary)">{progressLabel}</span>
        {/if}
      {:else if status === 'error'}
        <Banner tone="critical" title={description ?? ''} />
      {:else}
        {#if description}
          <p class="body" style:color="var(--text-secondary)">{description}</p>
        {/if}
      {/if}

      {#if !inProgress && warnings.length > 0}
        <div class="warnings">
          {#if warningsTitle}
            <span class="caption" style:color="var(--text-tertiary)">{warningsTitle}</span>
          {/if}
          <ul class="warning-list">
            {#each warnings as w (w)}
              <li class="body" style:color="var(--text-secondary)">{w}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button variant="secondary" label={closeLabel} disabled={inProgress} onclick={onClose} />
    {#if status === 'success' && onOpenSession && openSessionLabel}
      <Button variant="primary" label={openSessionLabel} onclick={openSession} />
    {/if}
  {/snippet}
</Dialog>

<style>
  .tw-import-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-import-body p {
    margin: 0;
  }
  .warnings {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    border: 1px solid color-mix(in srgb, var(--status-warm) 40%, var(--hairline));
    border-radius: var(--radius-md);
    padding: var(--space-3);
    background: color-mix(in srgb, var(--status-warm) 8%, var(--surface));
  }
  .warning-list {
    margin: 0;
    padding-left: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
