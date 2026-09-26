<script lang="ts">
  import { onMount } from 'svelte';
  import GuidedDiagnosticScreen from '../../design-system/components/GuidedDiagnosticScreen.svelte';
  import {
    commandResponseSchemas,
    type GuidedPhase,
    type GuidedPreflight,
    type SessionSummary
  } from '../../lib/bridge/schemas';
  import {
    invokeValidated,
    listenValidated,
    parseEvent
  } from '../../lib/bridge';
  import { getApplicationLogger } from '../../lib/logging';
  import { getTranslator } from '../../lib/i18n/runtime';
  import {
    classificationLabelFor,
    classificationOf
  } from '../sessions/classification';
  import { requestSessionReport } from '../sessions/handoff';
  import {
    batteryStateOf,
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
  let preflight = $state<GuidedPreflight | null>(null);
  // `guided.duration` and `guided.require_ac` from Ajustes. Both used to be ignored: the screen
  // always started the standard profile and always demanded AC, whatever was chosen there.
  let profile = $state<'short' | 'standard' | 'long'>('standard');
  let requireAc = $state(false);
  let failed = $state(false);
  let finished = $state<SessionSummary | null>(null);
  let finishedSessionId = $state<string | null>(null);
  const { t } = getTranslator();

  let checks = $derived(
    preflight === null ? [] : preflightChecks(preflight, t, requireAc)
  );
  let batteryState = $derived(batteryStateOf(preflight, requireAc));

  function reportFailure(action: string, messageKey: string): void {
    failed = true;
    getApplicationLogger().warn(`guided ${action} failed: ${messageKey}`);
  }

  async function loadPreferences(): Promise<void> {
    const result = await invokeValidated(
      'get_preferences',
      undefined,
      commandResponseSchemas.get_preferences
    );
    if (!result.ok) return;
    const duration = result.value.values['guided.duration'];
    if (
      duration === 'short' ||
      duration === 'standard' ||
      duration === 'long'
    ) {
      profile = duration;
    }
    requireAc = result.value.values['guided.require_ac'] === true;
  }

  async function loadPreflight(): Promise<void> {
    await loadPreferences();
    const result = await invokeValidated(
      'get_guided_preflight',
      undefined,
      commandResponseSchemas.get_guided_preflight
    );
    if (result.ok) preflight = result.value;
    else reportFailure('preflight', result.error.message_key);
  }

  async function start(skipRest = false): Promise<void> {
    const result = await invokeValidated(
      'start_guided',
      {
        request: { profile, skip_rest: skipRest, require_ac: requireAc }
      },
      commandResponseSchemas.start_guided
    );
    if (result.ok) {
      failed = false;
      finished = null;
      finishedSessionId = null;
      guidedState = result.value;
      publishGuidedPhase(result.value);
    } else reportFailure('start', result.error.message_key);
  }

  async function loadFinished(sessionId: string): Promise<void> {
    const result = await invokeValidated(
      'get_session',
      { request: { session_id: sessionId } },
      commandResponseSchemas.get_session
    );
    if (result.ok && finishedSessionId === sessionId) {
      finished = result.value.summary;
    }
  }

  async function markAsReference(): Promise<void> {
    if (finished === null) return;
    const result = await invokeValidated(
      'set_session_reference',
      { request: { session_id: finished.session_id, is_reference: true } },
      commandResponseSchemas.set_session_reference
    );
    if (result.ok) await loadFinished(finished.session_id);
    else reportFailure('reference', result.error.message_key);
  }

  function leave(): void {
    const sessionId = finishedSessionId;
    const showReport = sessionId !== null && currentPhase === 'result';
    guidedState = null;
    finished = null;
    finishedSessionId = null;
    failed = false;
    publishGuidedPhase(null);
    void loadPreflight();
    if (showReport) requestSessionReport(sessionId);
    window.dispatchEvent(
      new CustomEvent('throttlewatch:navigate', {
        detail: { destination: showReport ? 'sessions' : 'now' }
      })
    );
  }

  async function stop(): Promise<void> {
    await stopGuidedSession();
  }

  function onFocus(): void {
    if (guidedState === null) void loadPreflight();
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
    window.addEventListener('focus', onFocus);
    let unlisten: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;
    let unlistenFrozen: (() => void) | undefined;
    void listenValidated('guided:phase', (value) => {
      const parsed = parseEvent('guided:phase', value);
      if (!parsed.ok) return;
      guidedState = parsed.value;
      publishGuidedPhase(parsed.value);
    }).then((stopListening) => (unlisten = stopListening));
    void listenValidated('guided:finished', (value) => {
      const parsed = parseEvent('guided:finished', value);
      if (!parsed.ok) return;
      finishedSessionId = parsed.value.session_id;
      void loadFinished(parsed.value.session_id);
    }).then((stopListening) => (unlistenFinished = stopListening));
    void listenValidated('report:frozen', (value) => {
      const parsed = parseEvent('report:frozen', value);
      if (parsed.ok && parsed.value.session_id === finishedSessionId) {
        void loadFinished(parsed.value.session_id);
      }
    }).then((stopListening) => (unlistenFrozen = stopListening));
    return () => {
      unsubscribeSession();
      window.removeEventListener('keydown', onShortcut);
      window.removeEventListener('focus', onFocus);
      unlisten?.();
      unlistenFinished?.();
      unlistenFrozen?.();
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
  let result = $derived(
    currentPhase === 'result' &&
      finished !== null &&
      finished.report_classification !== null
      ? {
          classification: classificationOf(finished.report_classification),
          classificationLabel: classificationLabelFor(
            t,
            finished.report_classification
          ),
          summarySentence: t('guided.resultSaved'),
          evidenceLine:
            finished.duration_ms === null
              ? undefined
              : `${t('guided.duration')}: ${String(Math.round(finished.duration_ms / 1000))} ${t('common.seconds')}`
        }
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
  preflightFailedTitle={t('guided.reviewConditions')}
  {batteryState}
  batteryMessage={requireAc
    ? t('guided.needsAc')
    : t('guided.onBatteryWarning')}
  preflightCheckingLabel={t('guided.checkingSensors')}
  readyTitle={t('guided.readyTitle')}
  readyBody={t('guided.readyBody')}
  introTitle={t('guided.title')}
  introBody={failed ? t('guided.errorDescription') : t('guided.introBody')}
  whatWillHappenTitle={t('guided.whatWillHappen')}
  whatWillHappen={[
    { label: t('guided.load'), value: t('guided.fixedLoop') },
    { label: t('guided.duration'), value: t(`guided.${profile}Profile`) }
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
  errorDescription={t('guided.errorDescription')}
  {result}
  useAsReferenceLabel={finished !== null && !finished.is_reference
    ? t('sessions.useReference')
    : undefined}
  onUseAsReference={() => void markAsReference()}
  referenceNote={finished?.is_reference ? t('guided.referenceSet') : undefined}
  closeLabel={currentPhase === 'result' && finishedSessionId !== null
    ? t('guided.viewReport')
    : t('guided.close')}
  restartLabel={t('guided.restart')}
  onRestart={() => void start()}
  onClose={leave}
/>
