<script lang="ts">
  /**
   * "Novedades" after an important update (FR-037, Historia 8 esc. 7):
   * a short stack of cards rendered over "Ahora" — title, one
   * sentence, an optional action ("Ir a Ajustes") and "Entendido".
   * It never re-runs the onboarding and never blocks detection: the
   * shell mounts it above the screen content, not as a modal.
   *
   * The host owns which cards are pending (versioned by
   * `onboarding_state.last_seen_notice_version`) and removes a card
   * from `cards` when `onDismiss(id)` fires; this component holds no
   * state of its own.
   */
  import Button from './Button.svelte';

  export interface WhatsNewCard {
    id: string;
    title: string;
    body: string;
    actionLabel?: string;
    onAction?: () => void;
  }

  interface Props {
    /** Section heading, e.g. "Novedades de la versión 1.5". */
    title?: string;
    cards: WhatsNewCard[];
    dismissLabel: string;
    onDismiss: (id: string) => void;
    dismissAllLabel?: string;
    onDismissAll?: () => void;
  }

  let { title, cards, dismissLabel, onDismiss, dismissAllLabel, onDismissAll }: Props = $props();
</script>

{#if cards.length > 0}
  <section class="tw-ds tw-whats-new" aria-label={title}>
    {#if title || (onDismissAll && dismissAllLabel)}
      <div class="head">
        {#if title}
          <h3 class="label" style:color="var(--text-tertiary)">{title}</h3>
        {/if}
        {#if onDismissAll && dismissAllLabel && cards.length > 1}
          <button type="button" class="caption dismiss-all" onclick={onDismissAll}>{dismissAllLabel}</button>
        {/if}
      </div>
    {/if}
    <div class="cards">
      {#each cards as card, i (card.id)}
        <article class="card tw-enter" style:--tw-i={i}>
          <span class="new-dot" aria-hidden="true"></span>
          <div class="card-text">
            <span class="body-strong" style:color="var(--text-primary)">{card.title}</span>
            <span class="body" style:color="var(--text-secondary)">{card.body}</span>
          </div>
          <div class="card-actions">
            {#if card.onAction && card.actionLabel}
              <Button variant="secondary" label={card.actionLabel} onclick={card.onAction} />
            {/if}
            <Button variant="primary" label={dismissLabel} onclick={() => onDismiss(card.id)} />
          </div>
        </article>
      {/each}
    </div>
  </section>
{/if}

<style>
  .tw-whats-new {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .head h3 {
    margin: 0;
  }
  .dismiss-all {
    border: none;
    background: transparent;
    color: var(--accent-blue);
    padding: 2px 4px;
    border-radius: var(--radius-sm);
  }
  .dismiss-all:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .card {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
  }
  .new-dot {
    width: 8px;
    height: 8px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--accent-blue);
    flex: 0 0 auto;
  }
  .card-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .card-actions {
    display: flex;
    gap: var(--space-2);
    flex: 0 0 auto;
  }
  @media (max-width: 599px) {
    .card {
      flex-wrap: wrap;
    }
    .card-actions {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
