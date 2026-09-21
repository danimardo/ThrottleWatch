<script lang="ts">
  /**
   * Reference composition for the full onboarding flow — NOT a
   * component this system exports. `OnboardingFlow` itself has no
   * business logic (no real sensor detection), so this example
   * simulates the domain states a host app would actually drive it
   * with: detección en curso / cobertura completa / cobertura parcial,
   * acceso avanzado presente/ausente, and a resumed session. The
   * control panel below the phone-sized flow frame exists ONLY for
   * this demo — a real app would drive these from its own detection
   * logic, not from buttons next to the flow.
   */
  import { OnboardingFlow, SegmentedControl, Switch, Button, Select } from '../components';
  import type { OnboardingStepContent, OnboardingDetectionContent, DetectionStatus, AdvancedAccessState } from '../components';

  let detectionStatus: DetectionStatus = $state('detecting');
  let advancedAccess: AdvancedAccessState = $state('installable');
  const ACCESS_NOTES: Record<AdvancedAccessState, string> = {
    not_needed: 'No hace falta acceso avanzado en este equipo.',
    available: 'Acceso avanzado disponible.',
    installable: 'Recomendado: el acceso avanzado sube este equipo al nivel A (confirmar la causa y estimar cuánto ayudaría enfriar mejor).',
    upgradable: 'Hay una versión anterior del controlador de acceso avanzado; actualizarla mejora la cobertura.',
    denied: 'El acceso avanzado está bloqueado por una directiva del sistema o por el antivirus.',
    error: 'No se pudo comprobar el acceso avanzado.'
  };
  let resumed = $state(false);
  let lastAction = $state('(ninguna todavía)');

  // $derived.by (the function form) rather than a bare $derived(expr):
  // TypeScript's control-flow narrowing otherwise sees `detectionStatus`
  // freshly initialized to the literal 'detecting' with no reassignment
  // between that line and this one (the reassignment inside `onRetry`
  // below doesn't count — it's a separate closure), and narrows every
  // `detectionStatus === '...'` comparison here to a same-literal
  // comparison, which it then flags as always-false. Wrapping the whole
  // expression in a function resets that narrowing (the standard fix
  // for this well-known TS false positive), independent of Svelte.
  let steps: [
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingDetectionContent
  ] = $derived.by(() => [
    {
      title: 'Entiende si el calor está limitando tu CPU',
      body: 'ThrottleWatch observa temperatura, carga y frecuencia activa para saber cuándo el calor frena tu procesador — nunca cambia nada por ti.'
    },
    {
      title: 'Qué observa',
      body: 'Cuatro señales en paralelo: temperatura, carga, frecuencia activa y potencia, cada una con su propio margen de confianza.'
    },
    {
      title: 'Qué puede concluir',
      body: 'Una observación no es una confirmación. ThrottleWatch distingue entre lo que mide, lo que infiere y lo que confirma con evidencia sostenida.'
    },
    {
      title: 'Privacidad y decisiones',
      body: 'Todo el procesamiento ocurre en este equipo. ThrottleWatch nunca envía tus lecturas a ningún servidor.'
    },
    {
      title: 'Este equipo',
      body: 'Estamos detectando los sensores disponibles en tu hardware para calibrar las lecturas.',
      status: detectionStatus,
      detectingLabel: 'Detectando sensores…',
      coverageTitle: detectionStatus === 'complete' ? 'Cobertura completa' : 'Cobertura parcial',
      coverageDescription:
        detectionStatus === 'complete'
          ? 'Sensor de CPU · Sensor de placa · Acceso WMI'
          : 'No se encontró el sensor de placa base — las lecturas de potencia usarán una estimación.',
      advancedAccess,
      advancedAccessNote: ACCESS_NOTES[advancedAccess],
      onRetry: () => {
        detectionStatus = 'detecting';
        lastAction = 'reintentar detección';
      },
      retryLabel: 'Reintentar',
      onRequestAdvancedAccess: () => {
        advancedAccess = 'available';
        lastAction = 'instalar acceso avanzado';
      },
      requestAccessLabel: 'Instalar acceso avanzado',
      onAccessRetry: () => {
        advancedAccess = 'available';
        lastAction = 'reintentar comprobación de acceso';
      },
      accessRetryLabel: 'Reintentar'
    }
  ]);
</script>

<div class="tw-ds onboarding-demo">
  <div class="control-panel">
    <div class="control">
      <span class="label" style:color="var(--text-tertiary)">DETECCIÓN (paso 5)</span>
      <SegmentedControl
        label="Estado de detección"
        value={detectionStatus}
        onchange={(v) => (detectionStatus = v as DetectionStatus)}
        options={[
          { value: 'detecting', label: 'En curso' },
          { value: 'complete', label: 'Completa' },
          { value: 'partial', label: 'Parcial' }
        ]}
      />
    </div>
    <div class="control">
      <span class="label" style:color="var(--text-tertiary)">ACCESO AVANZADO</span>
      <Select
        label="Estado del acceso avanzado"
        value={advancedAccess}
        onchange={(v) => (advancedAccess = v as AdvancedAccessState)}
        options={[
          { value: 'not_needed', label: 'No hace falta' },
          { value: 'available', label: 'Disponible' },
          { value: 'installable', label: 'Instalable' },
          { value: 'denied', label: 'Bloqueado' },
          { value: 'error', label: 'Error' }
        ]}
      />
    </div>
    <div class="control">
      <span class="label" style:color="var(--text-tertiary)">SESIÓN REANUDADA</span>
      <Switch bind:checked={resumed} label="Reanudar en el paso 5" />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="flow-frame">
    {#key resumed}
      <OnboardingFlow
        {steps}
        initialStep={resumed ? 4 : 0}
        resumedNote={resumed ? 'Retomando donde lo dejaste' : undefined}
        backLabel="Atrás"
        nextLabel="Siguiente"
        finishLabel="Empezar"
        onSkip={() => (lastAction = 'omitir onboarding')}
        skipLabel="Omitir"
        onFinish={() => (lastAction = 'finalizar onboarding')}
        progressLabel={(step, count) => `Paso ${step} de ${count}`}
      />
    {/key}
  </div>
</div>

<style>
  .onboarding-demo {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-8);
    align-items: flex-start;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .control-panel {
    width: 220px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
  }
  .control {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .last-action {
    padding-top: var(--space-2);
    border-top: 1px solid var(--hairline);
  }
  .flow-frame {
    width: 640px;
    min-height: 560px;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-8) var(--space-6);
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
