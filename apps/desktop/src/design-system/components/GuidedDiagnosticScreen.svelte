<script lang="ts">
  /**
   * The "Diagnóstico guiado" screen: a single component driven entirely
   * by the `phase` prop, covering every phase the spec lists —
   * explicación previa (`'intro'`), preflight de sensores y
   * alimentación (`'preflight'`), listo (`'ready'`), reposo opcional
   * (`'rest'`, omitible con `onSkipRest`), calentamiento
   * (`'warming'`), carga estable (`'steadyLoad'`), recuperación
   * (`'recovery'`), cancelando/cancelado (`'cancelling'`/`'cancelled'`),
   * parada de seguridad (`'safetyStop'`), sensor perdido
   * (`'sensorLost'`), error (`'error'`), and resultado final
   * (`'result'`).
   *
   * Presentational only: the component never runs a real diagnostic,
   * never decides when a phase transition happens, and never computes
   * a temperature, a limit, or a classification. Every phase's content
   * is a prop; `onStart`/`onStop`/`onPreflightRetry`/`onSensorRetry`/
   * `onRetry`/`onRestart`/`onClose` all just report the person's
   * intent — the host owns the real state machine and picks the next
   * `phase` value.
   *
   * "Detener ahora" (`onStop`/`stopLabel`) renders — visible and
   * enabled — for every phase where a test is actually running
   * (`preflight`/`ready`/`warming`/`steadyLoad`/`recovery`), stays
   * visible but disabled during `'cancelling'` (already stopping), and
   * is absent everywhere else (`intro`, and the five terminal states)
   * because there is nothing left to stop. That mapping is a fixed
   * rendering rule of this component, the same category of thing as
   * `OnboardingFlow` deciding "Omitir" only shows on slides 1–4 — it
   * is not the host's domain logic being duplicated here.
   *
   * No copy lives in this component either (rule 2) — the 6 stepper
   * labels are the required `stepLabels` prop (fixed order:
   * Comprobación, Reposo, Calentamiento, Carga sostenida, Recuperación,
   * Resultado), and every other visible string (the preflight-failed
   * banner, the "comprobando sensores" label, the per-reading StatWidget
   * captions, the "Qué va a pasar" list) is its own prop.
   *
   * The intro is structured (HU-04): besides `introTitle`/`introBody`
   * the host passes `whatWillHappen` — load type, total duration,
   * sensors used, automatic stop conditions, "not a benchmark" — so
   * the person consents to something concrete, not a paragraph.
   */
  import type { Snippet } from 'svelte';
  import type { Classification } from '../lib/classification';
  import StatusChip from './StatusChip.svelte';
  import StatWidget from './StatWidget.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Banner from './Banner.svelte';
  import Button from './Button.svelte';

  export type DiagnosticPhase =
    | 'intro'
    | 'preflight'
    | 'ready'
    | 'rest'
    | 'warming'
    | 'steadyLoad'
    | 'recovery'
    | 'cancelling'
    | 'cancelled'
    | 'safetyStop'
    | 'sensorLost'
    | 'error'
    | 'result';

  export type BatteryState = 'ok' | 'warning' | 'blocked';

  export interface PreflightCheck {
    label: string;
    status: 'checking' | 'ok' | 'failed';
    detail?: string;
  }

  export interface DiagnosticLiveReading {
    temperatureLabel: string;
    temperatureFootnote?: string;
    limitLabel: string;
    limitFootnote?: string;
    /** e.g. "12 °C" — margin to the limit; rendered as a third StatWidget when present. */
    headroomLabel?: string;
    headroomFootnote?: string;
    /** e.g. "Quedan 1 min 20 s" — shown next to the phase progress bar. */
    remainingLabel?: string;
    /** 0-100; omit for an indeterminate phase progress bar. */
    progressPercent?: number;
  }

  export interface WhatWillHappenItem {
    /** e.g. "Carga" */
    label: string;
    /** e.g. "Todos los núcleos, sin instrucciones AVX extremas" */
    value: string;
  }

  export interface DiagnosticResult {
    classification: Classification;
    classificationLabel: string;
    /** "resultado en una frase" */
    summarySentence: string;
    evidenceLine?: string;
  }

  interface Props {
    phase: DiagnosticPhase;
    title: string;

    /** 6 labels in fixed order — Comprobación, Reposo, Calentamiento, Carga sostenida, Recuperación, Resultado — for the internal stepper. */
    stepLabels?: [string, string, string, string, string, string];
    stepperLabel?: string;

    batteryState?: BatteryState;
    batteryMessage?: string;

    stopLabel?: string;
    onStop?: () => void;

    introTitle?: string;
    introBody?: string;
    /** Heading for the structured "Qué va a pasar" list. */
    whatWillHappenTitle?: string;
    whatWillHappen?: WhatWillHappenItem[];
    /** e.g. "No es un benchmark homologado: sirve para comparar este equipo consigo mismo." */
    introDisclaimer?: string;
    startLabel?: string;
    onStart?: () => void;

    /** `'rest'` phase: optional stabilisation wait the person may skip. */
    restTitle?: string;
    restDescription?: string;
    skipRestLabel?: string;
    onSkipRest?: () => void;

    preflightChecks?: PreflightCheck[];
    preflightFailedTitle?: string;
    preflightCheckingLabel?: string;
    preflightRetryLabel?: string;
    onPreflightRetry?: () => void;

    readyTitle?: string;
    readyBody?: string;

    phaseTitle?: string;
    phaseDescription?: string;
    reading?: DiagnosticLiveReading;
    temperatureStatLabel?: string;
    limitStatLabel?: string;
    headroomStatLabel?: string;

    cancellingLabel?: string;
    cancelledTitle?: string;
    cancelledDescription?: string;

    safetyStopTitle?: string;
    safetyStopDescription?: string;

    sensorLostTitle?: string;
    sensorLostDescription?: string;
    onSensorRetry?: () => void;
    sensorRetryLabel?: string;

    errorTitle?: string;
    errorDescription?: string;
    onRetry?: () => void;
    retryLabel?: string;

    result?: DiagnosticResult;
    /** "Usar como referencia" on the result; host persists via `set_session_reference`. */
    useAsReferenceLabel?: string;
    onUseAsReference?: () => void;
    /** Short line under the reference button, e.g. why it is (not) recommended. */
    referenceNote?: string;

    onClose?: () => void;
    closeLabel?: string;
    onRestart?: () => void;
    restartLabel?: string;
  }

  let {
    phase,
    title,
    stepLabels = ['', '', '', '', '', ''],
    stepperLabel,
    batteryState = 'ok',
    batteryMessage,
    stopLabel,
    onStop,
    introTitle,
    introBody,
    whatWillHappenTitle,
    whatWillHappen = [],
    introDisclaimer,
    startLabel,
    onStart,
    restTitle,
    restDescription,
    skipRestLabel,
    onSkipRest,
    preflightChecks = [],
    preflightFailedTitle,
    preflightCheckingLabel,
    preflightRetryLabel,
    onPreflightRetry,
    readyTitle,
    readyBody,
    phaseTitle,
    phaseDescription,
    reading,
    temperatureStatLabel,
    limitStatLabel,
    headroomStatLabel,
    cancellingLabel,
    cancelledTitle,
    cancelledDescription,
    safetyStopTitle,
    safetyStopDescription,
    sensorLostTitle,
    sensorLostDescription,
    onSensorRetry,
    sensorRetryLabel,
    errorTitle,
    errorDescription,
    onRetry,
    retryLabel,
    result,
    useAsReferenceLabel,
    onUseAsReference,
    referenceNote,
    onClose,
    closeLabel,
    onRestart,
    restartLabel
  }: Props = $props();

  const STEPPER_PHASES: DiagnosticPhase[] = ['preflight', 'ready', 'rest', 'warming', 'steadyLoad', 'recovery', 'result'];
  const STEP_INDEX: Partial<Record<DiagnosticPhase, number>> = {
    preflight: 0,
    ready: 0,
    rest: 1,
    warming: 2,
    steadyLoad: 3,
    recovery: 4,
    result: 5
  };
  const STEP_IDS = ['preflight', 'rest', 'warming', 'steadyLoad', 'recovery', 'result'] as const;

  let showStepper = $derived(STEPPER_PHASES.includes(phase));
  let currentStepIndex = $derived(STEP_INDEX[phase] ?? -1);

  let runningPhases: DiagnosticPhase[] = ['preflight', 'ready', 'rest', 'warming', 'steadyLoad', 'recovery'];
  let showStop = $derived(runningPhases.includes(phase) || phase === 'cancelling');
  let stopDisabled = $derived(phase === 'cancelling');

  let showClosingActions = $derived(
    phase === 'cancelled' || phase === 'safetyStop' || phase === 'sensorLost' || phase === 'error' || phase === 'result'
  );
</script>

{#snippet preflightRetryAction()}
  <Button variant="secondary" label={preflightRetryLabel ?? ''} onclick={onPreflightRetry} />
{/snippet}
{#snippet sensorRetryAction()}
  <Button variant="secondary" label={sensorRetryLabel ?? ''} onclick={onSensorRetry} />
{/snippet}
{#snippet errorRetryAction()}
  <Button variant="secondary" label={retryLabel ?? ''} onclick={onRetry} />
{/snippet}

<div class="tw-ds tw-guided-diagnostic">
  <div class="header">
    <h2 class="value-md" style:color="var(--text-primary)">{title}</h2>
    {#if showStop}
      <Button variant="secondary" label={stopLabel ?? ''} disabled={stopDisabled} onclick={onStop} />
    {/if}
  </div>

  {#if batteryState !== 'ok'}
    <Banner tone={batteryState === 'blocked' ? 'critical' : 'warning'} title={batteryMessage ?? ''} />
  {/if}

  {#if showStepper}
    <div class="stepper" role="list" aria-label={stepperLabel ?? ''}>
      {#each STEP_IDS as stepId, i (stepId)}
        <div class="step" role="listitem" class:done={i < currentStepIndex} class:current={i === currentStepIndex}>
          <span class="step-dot" aria-hidden="true"></span>
          <span class="caption step-label">{stepLabels[i]}</span>
        </div>
      {/each}
    </div>
  {/if}

  <div class="body">
    {#key phase}
    {#if phase === 'intro'}
      <div class="panel">
        <span class="body-strong" style:color="var(--text-primary)">{introTitle}</span>
        <p class="body" style:color="var(--text-secondary)">{introBody}</p>
        {#if whatWillHappen.length > 0}
          <div class="what-will-happen">
            {#if whatWillHappenTitle}
              <span class="caption" style:color="var(--text-tertiary)">{whatWillHappenTitle}</span>
            {/if}
            <dl class="wwh-list">
              {#each whatWillHappen as item (item.label)}
                <div class="wwh-row">
                  <dt class="caption" style:color="var(--text-tertiary)">{item.label}</dt>
                  <dd class="body" style:color="var(--text-primary)">{item.value}</dd>
                </div>
              {/each}
            </dl>
          </div>
        {/if}
        {#if introDisclaimer}
          <span class="caption" style:color="var(--text-tertiary)">{introDisclaimer}</span>
        {/if}
        <div class="panel-action">
          <Button variant="primary" label={startLabel ?? ''} disabled={batteryState === 'blocked'} onclick={onStart} />
        </div>
      </div>
    {:else if phase === 'preflight'}
      <div class="panel">
        <ul class="checklist">
          {#each preflightChecks as check (check.label)}
            <li class="check-row">
              <span class="check-icon" class:ok={check.status === 'ok'} class:failed={check.status === 'failed'} aria-hidden="true">
                {#if check.status === 'ok'}
                  <svg viewBox="0 0 16 16" width="14" height="14"><path d="M3 8.5 L6.5 12 L13 4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
                {:else if check.status === 'failed'}
                  <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 4 L12 12 M12 4 L4 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
                {:else}
                  <span class="check-spinner"></span>
                {/if}
              </span>
              <div class="check-text">
                <span class="body" style:color="var(--text-primary)">{check.label}</span>
                {#if check.detail}
                  <span class="caption" style:color="var(--text-tertiary)">{check.detail}</span>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
        {#if preflightChecks.some((c) => c.status === 'failed')}
          <Banner tone="warning" title={preflightFailedTitle ?? ''} action={onPreflightRetry ? preflightRetryAction : undefined} />
        {:else if preflightChecks.some((c) => c.status === 'checking')}
          <ProgressBar indeterminate tone="accent" label={preflightCheckingLabel} />
        {/if}
      </div>
    {:else if phase === 'ready'}
      <div class="panel">
        <span class="body-strong" style:color="var(--text-primary)">{readyTitle}</span>
        <p class="body" style:color="var(--text-secondary)">{readyBody}</p>
        <div class="panel-action">
          <Button variant="primary" label={startLabel ?? ''} disabled={batteryState === 'blocked'} onclick={onStart} />
        </div>
      </div>
    {:else if phase === 'rest'}
      <div class="panel">
        <span class="body-strong" style:color="var(--text-primary)">{restTitle}</span>
        {#if restDescription}
          <p class="body" style:color="var(--text-secondary)">{restDescription}</p>
        {/if}
        {#if reading}
          <div class="phase-progress">
            {#if reading.progressPercent !== undefined}
              <ProgressBar percent={reading.progressPercent} tone="accent" label={restTitle} />
            {:else}
              <ProgressBar indeterminate tone="accent" label={restTitle} />
            {/if}
            {#if reading.remainingLabel}
              <span class="caption" style:color="var(--text-tertiary)">{reading.remainingLabel}</span>
            {/if}
          </div>
        {/if}
        {#if onSkipRest && skipRestLabel}
          <div class="panel-action">
            <Button variant="secondary" label={skipRestLabel} onclick={onSkipRest} />
          </div>
        {/if}
      </div>
    {:else if phase === 'warming' || phase === 'steadyLoad' || phase === 'recovery'}
      <div class="panel">
        <span class="body-strong" style:color="var(--text-primary)">{phaseTitle}</span>
        {#if phaseDescription}
          <p class="body" style:color="var(--text-secondary)">{phaseDescription}</p>
        {/if}
        {#if reading}
          <div class="reading-grid">
            {#snippet tempIcon()}
              <svg viewBox="0 0 24 24" width="12" height="12"><path d="M12 3 a2.5 2.5 0 0 0-2.5 2.5 v8.7 a4 4 0 1 0 5 0 V5.5 A2.5 2.5 0 0 0 12 3Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" /></svg>
            {/snippet}
            {#snippet limitIcon()}
              <svg viewBox="0 0 24 24" width="12" height="12"><path d="M4 18 L10 10 L14 14 L20 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /><path d="M15 6 H20 V11" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
            {/snippet}
            {#snippet headroomIcon()}
              <svg viewBox="0 0 24 24" width="12" height="12"><path d="M12 5v14M6 11l6-6 6 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
            {/snippet}
            <StatWidget icon={tempIcon} tone="warm" label={temperatureStatLabel ?? ''} value={reading.temperatureLabel} footnote={reading.temperatureFootnote ?? ''} />
            <StatWidget icon={limitIcon} tone="unknown" label={limitStatLabel ?? ''} value={reading.limitLabel} footnote={reading.limitFootnote ?? ''} />
            {#if reading.headroomLabel}
              <StatWidget icon={headroomIcon} tone="accent" label={headroomStatLabel ?? ''} value={reading.headroomLabel} footnote={reading.headroomFootnote ?? ''} />
            {/if}
          </div>
          <div class="phase-progress">
            {#if reading.progressPercent !== undefined}
              <ProgressBar percent={reading.progressPercent} tone="accent" label={phaseTitle} />
            {:else}
              <ProgressBar indeterminate tone="accent" label={phaseTitle} />
            {/if}
            {#if reading.remainingLabel}
              <span class="caption" style:color="var(--text-tertiary)">{reading.remainingLabel}</span>
            {/if}
          </div>
        {/if}
      </div>
    {:else if phase === 'cancelling'}
      <div class="panel">
        <ProgressBar indeterminate tone="accent" label={cancellingLabel} />
        <span class="caption" style:color="var(--text-tertiary)">{cancellingLabel}</span>
      </div>
    {:else if phase === 'cancelled'}
      <Banner tone="info" title={cancelledTitle ?? ''} description={cancelledDescription} />
    {:else if phase === 'safetyStop'}
      <Banner tone="critical" title={safetyStopTitle ?? ''} description={safetyStopDescription} />
    {:else if phase === 'sensorLost'}
      <Banner tone="critical" title={sensorLostTitle ?? ''} description={sensorLostDescription} action={onSensorRetry ? sensorRetryAction : undefined} />
    {:else if phase === 'error'}
      <Banner tone="critical" title={errorTitle ?? ''} description={errorDescription} action={onRetry ? errorRetryAction : undefined} />
    {:else if phase === 'result' && result}
      <div class="panel result-panel">
        <StatusChip classification={result.classification} label={result.classificationLabel} />
        <span class="value-md" style:color="var(--text-primary)">{result.summarySentence}</span>
        {#if result.evidenceLine}
          <span class="body" style:color="var(--text-secondary)">{result.evidenceLine}</span>
        {/if}
        {#if onUseAsReference && useAsReferenceLabel}
          <div class="reference-block">
            <Button variant="secondary" label={useAsReferenceLabel} onclick={onUseAsReference} />
            {#if referenceNote}
              <span class="caption" style:color="var(--text-tertiary)">{referenceNote}</span>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    {/key}

    {#if showClosingActions}
      <div class="closing-actions">
        {#if onRestart}
          <Button variant="secondary" label={restartLabel ?? ''} onclick={onRestart} />
        {/if}
        {#if onClose}
          <Button variant="primary" label={closeLabel ?? ''} onclick={onClose} />
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .tw-guided-diagnostic {
    background: transparent;
    padding: var(--space-6);
    max-width: 640px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    container-type: inline-size;
    container-name: tw-guided-diagnostic;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .header h2 {
    margin: 0;
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .step {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    color: var(--text-tertiary);
  }
  .step-dot {
    width: 100%;
    height: 3px;
    border-radius: var(--radius-full);
    background: var(--surface-sunken);
    display: block;
  }
  .step-dot {
    transition: background-color var(--motion-slow) var(--motion-ease-out);
  }
  .step.done .step-dot {
    background: var(--accent-blue);
  }
  .step.current .step-dot {
    background: var(--accent-blue);
  }
  .step.current .step-label {
    color: var(--text-primary);
    font-weight: 650;
  }
  .step-label {
    text-align: center;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .panel {
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-lg);
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .panel p {
    margin: 0;
  }
  .panel-action {
    margin-top: var(--space-2);
  }
  .checklist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .check-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }
  .check-icon {
    flex: 0 0 auto;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
    background: var(--surface-sunken);
  }
  .check-icon.ok {
    color: var(--status-normal);
    background: color-mix(in srgb, var(--status-normal) 14%, transparent);
  }
  .check-icon.failed {
    color: var(--status-thermal);
    background: color-mix(in srgb, var(--status-thermal) 14%, transparent);
  }
  .check-spinner {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 2px solid var(--text-tertiary);
    border-top-color: transparent;
    animation: tw-spin 0.8s linear infinite;
  }
  @keyframes tw-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .check-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .reading-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: var(--space-3);
  }
  .phase-progress {
    margin-top: var(--space-1);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .what-will-happen {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
  }
  .wwh-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .wwh-row {
    display: grid;
    grid-template-columns: minmax(96px, 30%) 1fr;
    gap: var(--space-3);
  }
  .wwh-row dt,
  .wwh-row dd {
    margin: 0;
  }
  .reference-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-2);
  }
  .result-panel {
    align-items: flex-start;
  }
  .closing-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  @container tw-guided-diagnostic (max-width: 479px) {
    .header {
      flex-direction: column;
      align-items: stretch;
    }
    .wwh-row {
      grid-template-columns: 1fr;
      gap: 2px;
    }
    .step-label {
      font-size: 10px;
    }
  }
</style>
