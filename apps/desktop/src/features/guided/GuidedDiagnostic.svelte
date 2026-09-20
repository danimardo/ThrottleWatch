<script lang="ts">
  import { onMount } from 'svelte';
  import GuidedDiagnosticScreen from '../../design-system/components/GuidedDiagnosticScreen.svelte';
  import {
    commandResponseSchemas,
    type GuidedPhase
  } from '../../lib/bridge/schemas';
  import {
    invokeValidated,
    listenValidated,
    parseEvent
  } from '../../lib/bridge';
  import { createTranslator } from '../../lib/i18n';
  import {
    percentRemaining,
    preflightChecks,
    toDiagnosticPhase
  } from './model';
  import {
    publishGuidedPhase,
    stopGuidedSession,
    subscribeGuidedSession
  } from './session';

  let guidedState = $state<GuidedPhase | null>(null);
  let checks = $state<ReturnType<typeof preflightChecks>>([]);
  let error = $state<string | undefined>();
  const { t } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );

  async function loadPreflight(): Promise<void> {
    const result = await invokeValidated(
      'get_guided_preflight',
      undefined,
      commandResponseSchemas.get_guided_preflight
    );
    if (result.ok) checks = preflightChecks(result.value);
    else
      error = `${result.error.message_key}: ${JSON.stringify(result.error.details)}`;
  }

  async function start(skipRest = false): Promise<void> {
    const result = await invokeValidated(
      'start_guided',
      {
        request: { profile: 'standard', skip_rest: skipRest, require_ac: true }
      },
      commandResponseSchemas.start_guided
    );
    if (result.ok) {
      guidedState = result.value;
      publishGuidedPhase(result.value);
    } else
      error = `${result.error.message_key}: ${JSON.stringify(result.error.details)}`;
  }

  async function stop(): Promise<void> {
    await stopGuidedSession();
  }

  function onShortcut(event: KeyboardEvent): void {
    if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'x') {
      event.preventDefault();
      void stop();
    }
  }

  onMount(() => {
    void loadPreflight();
    const unsubscribeSession = subscribeGuidedSession((phase) => {
      guidedState = phase;
    });
    window.addEventListener('keydown', onShortcut);
    let unlisten: (() => void) | undefined;
    void listenValidated('guided:phase', (value) => {
      const parsed = parseEvent('guided:phase', value);
      if (!parsed.ok) return;
      guidedState = parsed.value;
      publishGuidedPhase(parsed.value);
    }).then((stopListening) => (unlisten = stopListening));
    return () => {
      unsubscribeSession();
      window.removeEventListener('keydown', onShortcut);
      unlisten?.();
    };
  });

  let currentPhase = $derived(
    guidedState ? toDiagnosticPhase(guidedState.phase) : 'intro'
  );
  let progress = $derived(
    guidedState
      ? percentRemaining(guidedState.elapsed_ms, guidedState.remaining_ms)
      : undefined
  );
  let temperatureLabel = $derived(
    guidedState?.temperature_c === null ||
      guidedState?.temperature_c === undefined
      ? t('common.noValue')
      : `${guidedState.temperature_c.toFixed(0)} ${t('dashboard.celsius')}`
  );
  let limitLabel = $derived(
    guidedState?.thermal_limit_c === null ||
      guidedState?.thermal_limit_c === undefined
      ? t('common.noValue')
      : `${guidedState.thermal_limit_c.toFixed(0)} ${t('dashboard.celsius')}`
  );
  let headroomLabel = $derived(
    guidedState?.temperature_c !== null &&
      guidedState?.temperature_c !== undefined &&
      guidedState?.thermal_limit_c !== null &&
      guidedState?.thermal_limit_c !== undefined
      ? `${(guidedState.thermal_limit_c - guidedState.temperature_c).toFixed(0)} ${t('dashboard.celsius')}`
      : undefined
  );
  let reading = $derived({
    temperatureLabel,
    limitLabel,
    headroomLabel,
    progressPercent: progress,
    remainingLabel:
      guidedState?.remaining_ms === null ||
      guidedState?.remaining_ms === undefined
        ? undefined
        : `${Math.ceil(guidedState.remaining_ms / 1000)} ${t('common.seconds')}`
  });
</script>

<GuidedDiagnosticScreen
  phase={currentPhase}
  title={t('guided.title')}
  stepLabels={[
    t('guided.steps.check'),
    t('guided.steps.rest'),
    t('guided.steps.warming'),
    t('guided.steps.load'),
    t('guided.steps.recovery'),
    t('guided.steps.result')
  ]}
  stepperLabel={t('guided.phasesLabel')}
  preflightChecks={checks}
  preflightFailedTitle={error ?? t('guided.reviewConditions')}
  preflightCheckingLabel={t('guided.checkingSensors')}
  readyTitle={t('guided.readyTitle')}
  readyBody={t('guided.readyBody')}
  introTitle={t('guided.title')}
  introBody={error ?? t('guided.introBody')}
  whatWillHappenTitle={t('guided.whatWillHappen')}
  whatWillHappen={[
    { label: t('guided.load'), value: t('guided.fixedLoop') },
    { label: t('guided.duration'), value: t('guided.standardProfile') }
  ]}
  introDisclaimer={t('guided.disclaimer')}
  startLabel={t('guided.start')}
  onStart={() => void start()}
  restTitle={t('guided.steps.rest')}
  restDescription={t('guided.restDescription')}
  skipRestLabel={t('guided.skipRest')}
  onSkipRest={() => void start(true)}
  stopLabel={t('guided.stop')}
  onStop={stop}
  phaseTitle={t('guided.running')}
  phaseDescription={t('guided.runningDescription')}
  temperatureStatLabel={t('guided.temperature')}
  limitStatLabel={t('guided.effectiveLimit')}
  headroomStatLabel={t('guided.headroom')}
  {reading}
  cancellingLabel={t('guided.cancelling')}
  cancelledTitle={t('guided.cancelledTitle')}
  cancelledDescription={t('guided.cancelledDescription')}
  safetyStopTitle={t('guided.safetyStopTitle')}
  safetyStopDescription={t('guided.safetyStopDescription')}
  sensorLostTitle={t('guided.sensorLostTitle')}
  sensorLostDescription={t('guided.sensorLostDescription')}
  errorTitle={t('guided.errorTitle')}
  errorDescription={error}
  closeLabel={t('guided.close')}
  restartLabel={t('guided.restart')}
  onRestart={() => void start()}
  onClose={() => undefined}
/>
