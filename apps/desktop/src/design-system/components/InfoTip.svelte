<script lang="ts">
  /**
   * A small "?" that explains, in plain words, what the thing next to it means — for people who do
   * not know what a "clock", a "coverage tier" or a "hatched core" is. It is a `Tooltip` with its
   * own trigger, so it opens on hover AND on keyboard focus and stays open while the pointer moves
   * onto the text.
   *
   * `text` is the explanation and `label` the accessible name of the trigger ("More information");
   * both come already translated. It draws no copy of its own: the "?" is a glyph, not a word.
   */
  import Tooltip from './Tooltip.svelte';

  interface Props {
    text: string;
    label: string;
    placement?: 'top' | 'bottom' | 'left' | 'right';
  }

  let { text, label, placement = 'bottom' }: Props = $props();
</script>

<Tooltip {placement}>
  {#snippet body()}
    <span class="tip-text">{text}</span>
  {/snippet}
  {#snippet children({ describedBy })}
    <button type="button" class="info-tip" aria-label={label} aria-describedby={describedBy}>?</button>
  {/snippet}
</Tooltip>

<style>
  .info-tip {
    width: 16px;
    height: 16px;
    padding: 0;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    border: 1px solid var(--hairline);
    background: transparent;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: 10px;
    line-height: 1;
    font-weight: 700;
    cursor: default;
    transition: background-color var(--motion-fast) var(--motion-ease-out);
  }
  .info-tip:hover {
    background: color-mix(in srgb, var(--text-secondary) 12%, transparent);
  }
  .info-tip:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }
  .tip-text {
    display: block;
    text-align: left;
    font-weight: 500;
    white-space: normal;
  }
</style>
