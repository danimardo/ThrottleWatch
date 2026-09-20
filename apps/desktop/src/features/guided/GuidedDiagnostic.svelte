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
      ? '—'
      : `${guidedState.temperature_c.toFixed(0)} °C`
  );
  let limitLabel = $derived(
    guidedState?.thermal_limit_c === null ||
      guidedState?.thermal_limit_c === undefined
      ? '—'
      : `${guidedState.thermal_limit_c.toFixed(0)} °C`
  );
  let headroomLabel = $derived(
    guidedState?.temperature_c !== null &&
      guidedState?.temperature_c !== undefined &&
      guidedState?.thermal_limit_c !== null &&
      guidedState?.thermal_limit_c !== undefined
      ? `${(guidedState.thermal_limit_c - guidedState.temperature_c).toFixed(0)} °C`
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
        : `${Math.ceil(guidedState.remaining_ms / 1000)} s`
  });
</script>

<GuidedDiagnosticScreen
  phase={currentPhase}
  title="Diagnóstico guiado"
  stepLabels={[
    'Comprobación',
    'Reposo',
    'Calentamiento',
    'Carga sostenida',
    'Recuperación',
    'Resultado'
  ]}
  stepperLabel="Fases del diagnóstico"
  preflightChecks={checks}
  preflightFailedTitle={error ?? 'Revisa las condiciones antes de empezar'}
  preflightCheckingLabel="Comprobando sensores"
  readyTitle="Listo para empezar"
  readyBody="El diagnóstico usa un generador sin escrituras de hardware."
  introTitle="Diagnóstico guiado"
  introBody={error ??
    'Mide el rendimiento sostenido del equipo bajo sus condiciones actuales.'}
  whatWillHappenTitle="Qué va a pasar"
  whatWillHappen={[
    { label: 'Carga', value: 'Bucle fijo sin AVX-512' },
    { label: 'Duración', value: 'Perfil estándar' }
  ]}
  introDisclaimer="No es un benchmark homologado; sirve para comparar este equipo consigo mismo."
  startLabel="Iniciar"
  onStart={() => void start()}
  restTitle="Reposo"
  restDescription="Estabiliza el equipo antes de la carga."
  skipRestLabel="Omitir reposo"
  onSkipRest={() => void start(true)}
  stopLabel="Detener ahora"
  onStop={stop}
  phaseTitle="Diagnóstico en curso"
  phaseDescription="Las lecturas se actualizan durante la prueba."
  temperatureStatLabel="Temperatura"
  limitStatLabel="Límite efectivo"
  headroomStatLabel="Margen"
  {reading}
  cancellingLabel="Cancelando…"
  cancelledTitle="Prueba incompleta"
  cancelledDescription="La sesión se ha detenido y queda registrada como incompleta."
  safetyStopTitle="Parada de seguridad"
  safetyStopDescription="La prueba se detuvo al detectar una condición de seguridad."
  sensorLostTitle="Sensor perdido"
  sensorLostDescription="No se pudo mantener la cobertura necesaria."
  errorTitle="No se pudo iniciar"
  errorDescription={error}
  closeLabel="Cerrar"
  restartLabel="Repetir"
  onRestart={() => void start()}
  onClose={() => undefined}
/>
