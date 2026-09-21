<script lang="ts">
  /**
   * The full 5-slide onboarding state machine: owns which slide is
   * showing, renders the matching illustration + `OnboardingSlide`
   * chrome for it, and wires Atrás/Siguiente/Omitir. Purely
   * presentational — it holds only "which slide am I on" as local
   * state; persisting where the person left off (for `initialStep`)
   * and reacting to `onFinish`/`onSkip` are the host app's job.
   *
   * The 5 slides are fixed in order (Bienvenida / Qué observa / Qué
   * puede concluir / Privacidad y decisiones / Este equipo) because
   * that's the order the 5 `Onboarding*` illustrations were designed
   * for — `steps` supplies each slide's copy in that same fixed order,
   * and the 5th entry is the richer `OnboardingDetectionContent` shape
   * because slide 5 is also where passive hardware detection starts.
   *
   * Skippable: pass `onSkip` and every slide except the last shows an
   * "Omitir" button (the last slide has nothing left to skip — its
   * primary action already finishes the flow). Resumed: pass
   * `initialStep` > 0 and `resumedNote`, and that note shows once, only
   * on the slide the person is resumed into.
   */
  import OnboardingWelcome from '../illustrations/OnboardingWelcome.svelte';
  import OnboardingSignals from '../illustrations/OnboardingSignals.svelte';
  import OnboardingConclusions from '../illustrations/OnboardingConclusions.svelte';
  import OnboardingPrivacy from '../illustrations/OnboardingPrivacy.svelte';
  import OnboardingThisComputer from '../illustrations/OnboardingThisComputer.svelte';
  import OnboardingSlide from './OnboardingSlide.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Banner from './Banner.svelte';
  import Button from './Button.svelte';
  import { untrack } from 'svelte';
  import type { AdvancedAccessState } from '../lib/access';

  export interface OnboardingStepContent {
    title: string;
    body: string;
  }

  export type DetectionStatus = 'detecting' | 'complete' | 'partial';

  export interface OnboardingDetectionContent extends OnboardingStepContent {
    status: DetectionStatus;
    /** Shown next to the indeterminate bar while `status === 'detecting'`. */
    detectingLabel?: string;
    /** Short line/banner title once detection settles, e.g. "Cobertura completa" / "Cobertura parcial". */
    coverageTitle?: string;
    /** e.g. "Sensor de CPU · Sensor de placa · Acceso WMI" or what's missing when partial. */
    coverageDescription?: string;
    /**
     * UI-side state of the low-level access (see lib/access.ts). Only
     * `installable`/`upgradable` render the request button; `denied`/`error` render
     * a warning/critical banner with `advancedAccessNote` and, for
     * `error`, an optional retry; `not_needed`/`available` render at
     * most a quiet status line. Omit to render nothing.
     */
    advancedAccess?: AdvancedAccessState;
    /** Host-translated sentence describing `advancedAccess`. */
    advancedAccessNote?: string;
    onRetry?: () => void;
    retryLabel?: string;
    onRequestAdvancedAccess?: () => void;
    requestAccessLabel?: string;
    onAccessRetry?: () => void;
    accessRetryLabel?: string;
  }

  interface Props {
    steps: [
      OnboardingStepContent,
      OnboardingStepContent,
      OnboardingStepContent,
      OnboardingStepContent,
      OnboardingDetectionContent
    ];
    /** 0-based. Start the flow already on this slide (a resumed session). */
    initialStep?: number;
    /** Shown once, only on the slide `initialStep` points to. */
    resumedNote?: string;
    backLabel: string;
    nextLabel: string;
    /** Primary button label on the last slide (replaces `nextLabel`), e.g. "Empezar". */
    finishLabel: string;
    onSkip?: () => void;
    skipLabel?: string;
    onFinish: () => void;
    /** Notifies the host after navigation; the host owns persistence and detection. */
    onStepChange?: (step: number) => void;
    /** aria-label for the progress dots, e.g. `(step, count) => `Paso ${step} de ${count}``. Falls back to plain digits. */
    progressLabel?: (step: number, count: number) => string;
  }

  let {
    steps,
    initialStep = 0,
    resumedNote,
    backLabel,
    nextLabel,
    finishLabel,
    onSkip,
    skipLabel,
    onFinish,
    onStepChange,
    progressLabel
  }: Props = $props();

  const ILLUSTRATIONS = [OnboardingWelcome, OnboardingSignals, OnboardingConclusions, OnboardingPrivacy, OnboardingThisComputer];
  const STEP_COUNT = 5;

  // Seeded once from `initialStep` (a resumed session's saved step),
  // then owned locally — that's what `untrack` signals here: this is
  // deliberately an initial value, not a prop this component tracks.
  let currentStep = $state(untrack(() => Math.max(0, Math.min(STEP_COUNT - 1, initialStep))));

  let currentIllustration = $derived(ILLUSTRATIONS[currentStep]);
  let currentContent = $derived(steps[currentStep] ?? steps[0]);
  let isLastStep = $derived(currentStep === STEP_COUNT - 1);
  let detection = $derived(isLastStep ? (currentContent as OnboardingDetectionContent) : null);

  function goBack() {
    if (currentStep > 0) {
      currentStep -= 1;
      onStepChange?.(currentStep);
    }
  }
  function goNext() {
    if (isLastStep) {
      onFinish();
    } else {
      currentStep += 1;
      onStepChange?.(currentStep);
    }
  }
</script>

{#snippet illustrationFor()}
  {@const Illustration = currentIllustration}
  <Illustration />
{/snippet}

{#snippet detectionExtra()}
  {#if detection}
    <div class="detection">
      {#if detection.status === 'detecting'}
        <div class="detecting-row">
          <span class="body" style:color="var(--text-secondary)">{detection.detectingLabel}</span>
          <ProgressBar indeterminate tone="accent" label={detection.detectingLabel} />
        </div>
      {:else if detection.status === 'complete'}
        <!--
          Deliberately not StatusIcon — its glyphs are the closed set for
          diagnostic_report.classification (see lib/classification.ts and
          Banner's own doc comment for the same reasoning). "Detection
          finished successfully" is a different domain, so this draws its
          own small checkmark, the same shape Select uses for its
          selected-option mark.
        -->
        <div class="coverage-line">
          <span class="glyph" style:color="var(--status-normal)" aria-hidden="true">
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </span>
          <div class="coverage-text">
            <span class="body-strong" style:color="var(--text-primary)">{detection.coverageTitle}</span>
            {#if detection.coverageDescription}
              <span class="body" style:color="var(--text-secondary)">{detection.coverageDescription}</span>
            {/if}
          </div>
        </div>
      {:else if detection.status === 'partial'}
        <Banner
          tone="warning"
          title={detection.coverageTitle ?? ''}
          description={detection.coverageDescription}
          action={detection.onRetry ? retryAction : undefined}
        />
      {/if}

      {#if detection.advancedAccess === 'installable' || detection.advancedAccess === 'upgradable'}
        <Banner
          tone="info"
          title={detection.advancedAccessNote ?? ''}
          action={detection.onRequestAdvancedAccess ? requestAccessAction : undefined}
        />
      {:else if detection.advancedAccess === 'denied'}
        <Banner tone="warning" title={detection.advancedAccessNote ?? ''} />
      {:else if detection.advancedAccess === 'error'}
        <Banner
          tone="critical"
          title={detection.advancedAccessNote ?? ''}
          action={detection.onAccessRetry ? accessRetryAction : undefined}
        />
      {:else if (detection.advancedAccess === 'available' || detection.advancedAccess === 'not_needed') && detection.advancedAccessNote}
        <span class="caption access-line" style:color="var(--text-tertiary)">{detection.advancedAccessNote}</span>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet retryAction()}
  {#if detection?.onRetry}
    <Button variant="secondary" label={detection.retryLabel ?? ''} onclick={detection.onRetry} />
  {/if}
{/snippet}

{#snippet requestAccessAction()}
  {#if detection?.onRequestAdvancedAccess}
    <Button variant="secondary" label={detection.requestAccessLabel ?? ''} onclick={detection.onRequestAdvancedAccess} />
  {/if}
{/snippet}

{#snippet accessRetryAction()}
  {#if detection?.onAccessRetry}
    <Button variant="secondary" label={detection.accessRetryLabel ?? ''} onclick={detection.onAccessRetry} />
  {/if}
{/snippet}

{#key currentStep}
<OnboardingSlide
  illustration={illustrationFor}
  title={currentContent.title}
  body={currentContent.body}
  stepIndex={currentStep}
  stepCount={STEP_COUNT}
  progressLabel={progressLabel?.(currentStep + 1, STEP_COUNT)}
  note={currentStep === initialStep && initialStep > 0 ? resumedNote : undefined}
  extra={isLastStep ? detectionExtra : undefined}
  onBack={currentStep > 0 ? goBack : undefined}
  {backLabel}
  onSkip={!isLastStep ? onSkip : undefined}
  skipLabel={skipLabel ?? ''}
  onNext={goNext}
  nextLabel={isLastStep ? finishLabel : nextLabel}
/>
{/key}

<style>
  .detection {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    text-align: left;
  }
  .detecting-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .coverage-line {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--surface-raised);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
  }
  .coverage-line .glyph {
    flex: 0 0 auto;
    width: 16px;
    height: 16px;
    display: flex;
    margin-top: 1px;
  }
  .coverage-line .glyph :global(svg) {
    width: 100%;
    height: 100%;
  }
  .coverage-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
