<script lang="ts">
  /**
   * The one modal primitive behind every confirmation in the spec:
   * the first-close "Salir de ThrottleWatch / Continuar en la
   * bandeja" choice, "Restablecer ThrottleWatch" / "Eliminar todos
   * mis datos" confirmations, and the update-install confirmation
   * that "anuncia el cierre". All of them are a title, a short body,
   * and 1–3 actions — never a form, never scrolling content.
   *
   * Built on the native <dialog> element (showModal/close) so focus
   * trapping, the backdrop, and Escape-to-dismiss come from the
   * platform instead of being reimplemented — Tauri's WebView2/
   * Chromium runtime supports it.
   *
   * `open` is bindable. Compose the action row yourself with `Button`
   * (see components/Button.svelte's rule on where "destructive" may
   * be used — only the confirm button in a dialog like this one):
   *
   *   <Dialog bind:open title="Restablecer ThrottleWatch"
   *     description="Se borrarán las preferencias y los datos. La próxima vez se repetirá el primer inicio.">
   *     {#snippet actions()}
   *       <Button variant="secondary" label="Cancelar" onclick={() => open = false} />
   *       <Button variant="destructive" label="Restablecer" onclick={confirmReset} />
   *     {/snippet}
   *   </Dialog>
   *
   * Tone "warning" adds a small triangle mark next to the title for
   * an irreversible action's confirmation — it does not change which
   * Button variant you use inside `actions`.
   */
  import type { Snippet } from 'svelte';
  import StatusIcon from '../icons/StatusIcon.svelte';

  interface Props {
    open?: boolean;
    title: string;
    description?: string;
    tone?: 'default' | 'warning';
    body?: Snippet;
    actions: Snippet;
    onclose?: () => void;
    /** `wide` (560px) for dialogs with lists or two columns, e.g. ExportDialog. Default stays 400px. */
    size?: 'default' | 'wide';
  }

  let { open = $bindable(false), title, description, tone = 'default', body, actions, onclose, size = 'default' }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  const titleId = `tw-dialog-title-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) {
      dialogEl.showModal();
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  function handleNativeClose() {
    open = false;
    onclose?.();
  }
</script>

<dialog bind:this={dialogEl} class="tw-dialog" class:wide={size === 'wide'} aria-labelledby={titleId} onclose={handleNativeClose}>
  <div class="header">
    {#if tone === 'warning'}
      <span class="tone-icon"><StatusIcon kind="warning" /></span>
    {/if}
    <h2 id={titleId} class="value-md">{title}</h2>
  </div>
  {#if body}
    {@render body()}
  {:else if description}
    <p class="body" style:color="var(--text-secondary)">{description}</p>
  {/if}
  <div class="actions">
    {@render actions()}
  </div>
</dialog>

<style>
  .tw-dialog {
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    color: var(--text-primary);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    width: min(400px, calc(100vw - var(--space-8)));
    max-width: 400px;
  }
  .tw-dialog.wide {
    width: min(560px, calc(100vw - var(--space-8)));
    max-width: 560px;
  }
  .tw-dialog[open] {
    animation: tw-pop var(--motion-base) var(--motion-spring) both;
  }
  .tw-dialog::backdrop {
    background: rgba(0, 0, 0, 0.32);
    -webkit-backdrop-filter: blur(6px);
    backdrop-filter: blur(6px);
    animation: tw-fade 240ms ease-out both;
  }
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: var(--space-2);
  }
  .header h2 {
    margin: 0;
    color: var(--text-primary);
  }
  .tone-icon {
    width: 18px;
    height: 18px;
    color: var(--status-warm);
    flex: 0 0 auto;
    display: flex;
  }
  .tone-icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  p.body {
    margin: 0 0 var(--space-5) 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
</style>
