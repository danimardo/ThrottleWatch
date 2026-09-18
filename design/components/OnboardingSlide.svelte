<script lang="ts">
  /**
   * The chrome around one onboarding illustration: title, body copy,
   * an optional step-specific `extra` block (slide 5's detection
   * status lives here — see OnboardingFlow), the progress dots, and the
   * Atrás/Omitir/Siguiente button row. `OnboardingFlow` is what actually
   * assembles the 5 slides in order; reach for this directly only if
   * you're building a custom flow that doesn't fit `OnboardingFlow`'s
   * shape.
   *
   * `onBack` omitted hides the "Atrás" button entirely (slide 1) rather
   * than rendering it disabled — there's nothing to go back to, so a
   * disabled control would just be visual noise. Same for `onSkip`.
   */
  import type { Snippet } from 'svelte';
  import OnboardingProgress from './OnboardingProgress.svelte';
  import Button from './Button.svelte';

  interface Props {
    illustration: Snippet;
    title: string;
    body: string;
    stepIndex: number;
    stepCount: number;
    progressLabel?: string;
    /** A small note shown above the title — e.g. "resuming where you left off". Omit for a normal slide. */
    note?: string;
    extra?: Snippet;
    onBack?: () => void;
    /** Required even if `onBack` is omitted this slide — simplest to always pass it and let presence of `onBack` decide whether it renders. */
    backLabel: string;
    onSkip?: () => void;
    skipLabel: string;
    onNext: () => void;
    nextLabel: string;
  }

  let {
    illustration,
    title,
    body,
    stepIndex,
    stepCount,
    progressLabel,
    note,
    extra,
    onBack,
    backLabel,
    onSkip,
    skipLabel,
    onNext,
    nextLabel
  }: Props = $props();
</script>

<div class="tw-onboarding-slide">
  {#if note}
    <span class="note label" style:color="var(--accent-blue)">{note}</span>
  {/if}

  <div class="illustration">
    {@render illustration()}
  </div>

  <div class="copy">
    <span class="value-md" style:color="var(--text-primary)">{title}</span>
    <span class="body" style:color="var(--text-secondary)">{body}</span>
  </div>

  {#if extra}
    <div class="extra">{@render extra()}</div>
  {/if}

  <div class="footer">
    <div class="footer-left">
      {#if onBack}
        <Button variant="secondary" label={backLabel} onclick={onBack} />
      {/if}
    </div>

    <OnboardingProgress {stepCount} currentStep={stepIndex} label={progressLabel} />

    <div class="footer-right">
      {#if onSkip}
        <Button variant="secondary" label={skipLabel} onclick={onSkip} />
      {/if}
      <Button variant="primary" label={nextLabel} onclick={onNext} />
    </div>
  </div>
</div>

<style>
  .tw-onboarding-slide {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    max-width: 420px;
    margin: 0 auto;
  }
  .illustration {
    animation: tw-pop var(--motion-slow) var(--motion-spring) both;
  }
  .copy {
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    animation-delay: var(--motion-stagger);
  }
  .note {
    margin-bottom: calc(-1 * var(--space-3));
  }
  .illustration {
    width: 280px;
    height: 220px;
  }
  .copy {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .extra {
    width: 100%;
  }
  .footer {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-top: var(--space-3);
  }
  .footer-left {
    flex: 1;
    display: flex;
    justify-content: flex-start;
  }
  .footer-right {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
