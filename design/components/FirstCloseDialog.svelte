<script lang="ts">
  /**
   * The first-close question (FR-046): the very first time the X is
   * pressed with `lifecycle.close_action = unset`, Rust emits
   * `lifecycle:close-decision-required` and the shell mounts this.
   * Two choices — exit, or keep running in the tray *and keep
   * measuring* — plus a hint that the choice can be changed in
   * Ajustes. Choosing the tray also enables background monitoring;
   * `trayNote` is where the host says so.
   *
   * Dismissing (Escape / backdrop) persists nothing and leaves the
   * window open: `onDismiss` maps to `resolve_first_close({ action:
   * "dismiss" })`. There is no "remember my choice" checkbox because
   * the choice is always remembered — that is the spec, not a
   * simplification.
   *
   * No copy lives here (rule 2).
   */
  import Dialog from './Dialog.svelte';
  import Button from './Button.svelte';

  interface Props {
    open?: boolean;
    title: string;
    description: string;
    exitLabel: string;
    trayLabel: string;
    /** Explains that "tray" turns on background monitoring. */
    trayNote?: string;
    /** "Puedes cambiarlo en Ajustes → General". */
    settingsHint?: string;
    onExit: () => void;
    onTray: () => void;
    onDismiss: () => void;
  }

  let { open = $bindable(false), title, description, exitLabel, trayLabel, trayNote, settingsHint, onExit, onTray, onDismiss }: Props =
    $props();

  // `Dialog.onclose` also fires after a programmatic close (the native
  // <dialog> `close` event does not distinguish), so an explicit choice
  // must not be followed by the dismiss callback. `decided` is reset
  // each time the dialog opens.
  let decided = $state(false);
  $effect(() => {
    if (open) decided = false;
  });

  function choose(fn: () => void) {
    decided = true;
    fn();
  }
  function handleClose() {
    if (!decided) onDismiss();
  }
</script>

<Dialog bind:open {title} onclose={handleClose}>
  {#snippet body()}
    <div class="tw-first-close-body">
      <p class="body" style:color="var(--text-secondary)">{description}</p>
      {#if trayNote}
        <p class="caption" style:color="var(--text-tertiary)">{trayNote}</p>
      {/if}
      {#if settingsHint}
        <p class="caption hint" style:color="var(--text-tertiary)">{settingsHint}</p>
      {/if}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button variant="secondary" label={exitLabel} onclick={() => choose(onExit)} />
    <Button variant="primary" label={trayLabel} onclick={() => choose(onTray)} />
  {/snippet}
</Dialog>

<style>
  .tw-first-close-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-first-close-body p {
    margin: 0;
  }
  .hint {
    padding-top: var(--space-2);
    border-top: 1px solid var(--hairline);
  }
</style>
