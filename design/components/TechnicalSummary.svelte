<script lang="ts">
  /**
   * The copyable technical summary (plan.md → "Observabilidad",
   * `get_technical_summary`): versions, protocol, collector state,
   * coverage, last error codes and local metrics — already stripped of
   * identifiers by the host. Rendered as monospaced, tabular text with
   * a single "Copiar" action; the host performs the clipboard write
   * and flips `copied` for a moment so the button can confirm it.
   *
   * Lives in Ajustes → Acerca de y ayuda, and is what the global
   * collector banner's "Ver resumen técnico" action opens.
   */
  import Button from './Button.svelte';

  interface Props {
    title?: string;
    /** Sentence saying the text contains no identifiers. */
    note?: string;
    text: string;
    copyLabel: string;
    copiedLabel?: string;
    copied?: boolean;
    onCopy: () => void;
    /** Accessible name for the read-only text region. */
    regionLabel: string;
  }

  let { title, note, text, copyLabel, copiedLabel, copied = false, onCopy, regionLabel }: Props = $props();
</script>

<div class="tw-ds tw-technical-summary">
  {#if title || note}
    <div class="head">
      {#if title}
        <span class="body-strong" style:color="var(--text-primary)">{title}</span>
      {/if}
      {#if note}
        <span class="caption" style:color="var(--text-tertiary)">{note}</span>
      {/if}
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -- scrollable read-only region must stay keyboard-reachable -->
  <pre class="text" aria-label={regionLabel} tabindex="0">{text}</pre>
  <div class="actions">
    <Button variant="secondary" label={copied && copiedLabel ? copiedLabel : copyLabel} onclick={onCopy} />
  </div>
</div>

<style>
  .tw-technical-summary {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .text {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-family: ui-monospace, 'Cascadia Mono', Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 260px;
    overflow: auto;
  }
  .text:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
