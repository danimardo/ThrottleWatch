<script lang="ts">
  /**
   * Reference composition exercising every phase `GuidedDiagnosticScreen`
   * accepts — NOT a component this system exports. A demo control
   * panel picks the phase directly (a real app would drive this from
   * its own state machine, transitioning automatically); everything
   * else below is a static prop set per phase.
   *
   * See the note on `$derived.by` in OnboardingFlow.example.svelte —
   * the same TypeScript control-flow narrowing false positive applies
   * here (`phase` is reassigned inside the Select's `onchange`
   * closure, invisible to a bare `$derived` read), so every derived
   * value below that compares `phase` against a literal uses
   * `$derived.by`.
   */
  import { GuidedDiagnosticScreen, Select } from '../components';
  import type { DiagnosticPhase, PreflightCheck, BatteryState } from '../components';

  const PHASE_OPTIONS = [
    { value: 'intro', label: 'Explicación previa' },
    { value: 'preflight', label: 'Preflight (comprobando)' },
    { value: 'ready', label: 'Listo' },
    { value: 'rest', label: 'Reposo (opcional)' },
    { value: 'warming', label: 'Calentamiento' },
    { value: 'steadyLoad', label: 'Carga estable' },
    { value: 'recovery', label: 'Recuperación' },
    { value: 'cancelling', label: 'Cancelando' },
    { value: 'cancelled', label: 'Cancelado' },
    { value: 'safetyStop', label: 'Parada de seguridad' },
    { value: 'sensorLost', label: 'Sensor perdido' },
    { value: 'error', label: 'Error' },
    { value: 'result', label: 'Resultado final' }
  ];

  const BATTERY_OPTIONS = [
    { value: 'ok', label: 'Con corriente' },
    { value: 'warning', label: 'Aviso de batería' },
    { value: 'blocked', label: 'Bloqueado por batería' }
  ];

  let phase: DiagnosticPhase = $state('steadyLoad');
  let batteryState: BatteryState = $state('ok');
  let lastAction = $state('(ninguna todavía)');

  let preflightChecks: PreflightCheck[] = $derived.by(() => [
    { label: 'Sensor de temperatura de CPU', status: 'ok' },
    { label: 'Sensor de placa base', status: phase === 'preflight' ? 'checking' : 'ok' },
    {
      label: 'Alimentación conectada',
      status: 'failed',
      detail: 'El equipo funciona con batería; conecta el cargador para continuar.'
    }
  ]);

  let reading = $derived.by(() => {
    if (phase === 'rest') {
      return { temperatureLabel: '52°', limitLabel: 'TjMax 100°', progressPercent: 40, remainingLabel: 'Quedan 36 s' };
    }
    if (phase === 'warming') {
      return { temperatureLabel: '68°', temperatureFootnote: 'Subiendo', limitLabel: '100°', limitFootnote: 'TjMax − TCC offset', headroomLabel: '32°', headroomFootnote: 'Margen', progressPercent: 35, remainingLabel: 'Quedan 58 s' };
    }
    if (phase === 'steadyLoad') {
      return { temperatureLabel: '100°', temperatureFootnote: 'En el límite (meseta)', limitLabel: '100°', limitFootnote: 'TjMax − TCC offset', headroomLabel: '0°', headroomFootnote: 'Margen', progressPercent: 55, remainingLabel: 'Quedan 1 min 48 s' };
    }
    if (phase === 'recovery') {
      return { temperatureLabel: '74°', temperatureFootnote: 'Bajando', limitLabel: '100°', limitFootnote: 'TjMax − TCC offset', headroomLabel: '26°', headroomFootnote: 'Margen', progressPercent: 70, remainingLabel: 'Quedan 36 s' };
    }
    return undefined;
  });

  let phaseTitle = $derived.by(() => {
    if (phase === 'warming') return 'Calentamiento';
    if (phase === 'steadyLoad') return 'Carga sostenida';
    if (phase === 'recovery') return 'Recuperación';
    return undefined;
  });

  let phaseDescription = $derived.by(() => {
    if (phase === 'warming') return 'Aplicando una carga creciente para observar cómo responde la temperatura.';
    if (phase === 'steadyLoad') return 'Carga constante tras el fin del turbo. Frecuencia activa 2,1 GHz (base 2,6 GHz) · rendimiento medido 1.184 op/s (inicial 1.520). Llegar al límite de temperatura no detiene la prueba: el procesador se protege solo.';
    if (phase === 'recovery') return 'Retirando la carga para medir cuánto tarda la temperatura en volver a un rango normal.';
    return undefined;
  });
</script>

<div class="tw-ds guided-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">FASE</span>
      <Select label="Fase" value={phase} options={PHASE_OPTIONS} onchange={(v) => (phase = v as DiagnosticPhase)} />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">BATERÍA</span>
      <Select label="Batería" value={batteryState} options={BATTERY_OPTIONS} onchange={(v) => (batteryState = v as BatteryState)} />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <GuidedDiagnosticScreen
      {phase}
      title="Diagnóstico guiado"
      stepLabels={['Comprobación', 'Reposo', 'Calentamiento', 'Carga sostenida', 'Recuperación', 'Resultado']}
      stepperLabel="Progreso del diagnóstico"
      {batteryState}
      batteryMessage={batteryState === 'blocked'
        ? 'Conecta el cargador para iniciar el diagnóstico guiado.'
        : batteryState === 'warning'
          ? 'Funcionando con batería. El diagnóstico puede detenerse si la batería baja demasiado.'
          : undefined}
      stopLabel="Detener ahora"
      onStop={() => {
        lastAction = 'detener ahora';
        phase = 'cancelling';
      }}
      introTitle="Antes de empezar"
      introBody="Este diagnóstico aplica una carga controlada durante unos minutos para observar cómo responde tu CPU al calor. Puedes detenerlo en cualquier momento."
      whatWillHappenTitle="Qué va a pasar"
      whatWillHappen={[
        { label: 'Carga', value: 'Todos los núcleos, progresiva, sin instrucciones AVX extremas' },
        { label: 'Duración', value: 'Unos 9 minutos (estándar): reposo 1 min · calentamiento 1,5 min · carga 4 min · recuperación 2 min' },
        { label: 'Sensores', value: 'Temperatura de paquete, frecuencia activa frente a base, potencia y razones de limitación (si hay acceso avanzado)' },
        { label: 'Se detiene sola si', value: 'La temperatura supera el límite en más de 2 °C, la frecuencia cae por debajo de la mitad de la base estando al límite, se pierde el sensor 3 s o el generador no responde 5 s. Llegar al límite no la detiene.' }
      ]}
      introDisclaimer="No es un benchmark homologado: sirve para comparar este equipo consigo mismo."
      startLabel="Iniciar diagnóstico"
      restTitle="Reposo para estabilizar"
      restDescription="Espera un momento sin carga para que la temperatura parta de un punto estable. Puedes omitirlo."
      skipRestLabel="Omitir reposo"
      onSkipRest={() => {
        lastAction = 'omitir reposo';
        phase = 'warming';
      }}
      onStart={() => {
        lastAction = 'iniciar diagnóstico';
        phase = 'preflight';
      }}
      {preflightChecks}
      preflightFailedTitle="No se pudieron completar todas las comprobaciones"
      preflightCheckingLabel="Comprobando sensores y alimentación"
      preflightRetryLabel="Reintentar comprobación"
      onPreflightRetry={() => (lastAction = 'reintentar preflight')}
      readyTitle="Todo listo"
      readyBody="Se comprobaron los sensores y la alimentación. Puedes iniciar el diagnóstico cuando quieras."
      {phaseTitle}
      {phaseDescription}
      {reading}
      temperatureStatLabel="Temperatura"
      limitStatLabel="Límite"
      headroomStatLabel="Margen"
      cancellingLabel="Deteniendo el diagnóstico…"
      cancelledTitle="Diagnóstico cancelado"
      cancelledDescription="Se detuvo antes de completar todas las fases. No se generó ningún informe."
      safetyStopTitle="Parada de seguridad"
      safetyStopDescription="La temperatura superó el límite del procesador en más de 2 °C: su control térmico no estaba actuando. Se detuvo la carga para proteger el equipo."
      sensorLostTitle="Se perdió la señal de un sensor"
      sensorLostDescription="No se pudo seguir leyendo la temperatura de la CPU durante unos segundos."
      sensorRetryLabel="Reintentar lectura"
      onSensorRetry={() => (lastAction = 'reintentar lectura de sensor')}
      errorTitle="No se pudo completar el diagnóstico"
      errorDescription="Ocurrió un error inesperado al aplicar la carga de prueba."
      retryLabel="Reintentar"
      onRetry={() => (lastAction = 'reintentar diagnóstico')}
      result={{
        classification: 'thermal_probable',
        classificationLabel: 'Limitación térmica probable',
        summarySentence: 'Tu CPU sostuvo el 78 % del rendimiento inicial de la carga de prueba: 14 puntos por el fin del turbo (esperado) y 8 por temperatura.',
        evidenceLine: 'Medido por el generador de carga · enfriar mejor: +10–20 % · confianza media (sin razones directas)'
      }}
      useAsReferenceLabel="Usar como referencia"
      onUseAsReference={() => (lastAction = 'usar como referencia')}
      referenceNote="Servirá para comparar «antes/después» con otra prueba igual (misma duración y alimentación)."
      onClose={() => (lastAction = 'cerrar diagnóstico')}
      closeLabel="Cerrar"
      onRestart={() => (lastAction = 'repetir diagnóstico')}
      restartLabel="Repetir"
    />
  </div>
</div>

<style>
  .guided-demo {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .control-panel {
    width: 220px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    position: sticky;
    top: var(--space-6);
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
  .screen-frame {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  @media (max-width: 900px) {
    .guided-demo {
      flex-direction: column;
    }
    .control-panel {
      width: auto;
      position: static;
    }
    .screen-frame {
      width: 100%;
    }
  }
</style>
