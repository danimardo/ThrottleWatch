<script lang="ts">
  /**
   * Shown when the window is asked to close while an operation is in
   * progress (`lifecycle:close-blocked` in application-commands.md).
   * The `reason` only picks the tone; the host supplies the wording
   * and decides which actions exist:
   *
   * - `guided`   → "Detener y salir" (confirm) / "Cancelar"
   * - `export`   → host usually waits ≤ 5 s; confirm = cancel export and exit
   * - `download` → confirm = discard the download and exit
   * - `install`  → no confirm at all: the window cannot close; only "Entendido"
   *
   * Pass `onConfirm` only when a confirm action exists — for `install`
   * leave it out and the dialog renders a single close button.
   */
  import Dialog from './Dialog.svelte';
  import Button from './Button.svelte';

  export type CloseBlockedReason = 'guided' | 'export' | 'download' | 'install';

  interface Props {
    open?: boolean;
    reason: CloseBlockedReason;
    title: string;
    description: string;
    confirmLabel?: string;
    onConfirm?: () => void;
    cancelLabel: string;
    onCancel: () => void;
  }

  let { open = $bindable(false), reason, title, description, confirmLabel, onConfirm, cancelLabel, onCancel }: Props = $props();

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
    onConfirm?.();
  }
  function handleClose() {
    if (!decided) onCancel();
  }
</script>

<Dialog bind:open {title} {description} tone={reason === 'install' ? 'default' : 'warning'} onclose={handleClose}>
  {#snippet actions()}
    <Button variant={onConfirm ? 'secondary' : 'primary'} label={cancelLabel} onclick={onCancel} />
    {#if onConfirm && confirmLabel}
      <Button variant={reason === 'guided' ? 'destructive' : 'primary'} label={confirmLabel} onclick={confirm} />
    {/if}
  {/snippet}
</Dialog>
