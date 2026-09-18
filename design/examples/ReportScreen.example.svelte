<script lang="ts">
  /**
   * Reference composition exercising every state `ReportScreen`
   * accepts — NOT a component this system exports. A demo control
   * panel toggles: loading, session-incomplete notice, reduced-
   * confidence notice, whether a cooling-potential figure is
   * available at all, and whether the optional `CausalRail` evidence
   * chain is shown (per that component's own "only when backed by a
   * real 2-4 node chain" rule — this demo turns it off entirely
   * rather than ever padding it).
   *
   * See OnboardingFlow.example.svelte's note on `$derived.by` for why
   * derived values compared against a `$state`-declared literal use
   * it below.
   */
  import { ReportScreen, SegmentedControl, Switch, Button } from '../components';
  import type { ReportComparisonMetric, Classification } from '../components';

  let loading = $state(false);
  let sessionIncomplete = $state(false);
  let reducedConfidence = $state(false);
  let provisional = $state(false);
  let showReevaluated = $state(false);
  let hasBaseline = $state(true);
  let showCausalChain = $state(true);

  type ClassificationPreset = 'thermal_confirmed' | 'mixed_limit' | 'normal';
  let classificationPreset: ClassificationPreset = $state('thermal_confirmed');
  let lastAction = $state('(ninguna todavía)');

  let classification = $derived.by((): Classification => classificationPreset);
  let classificationLabel = $derived.by(() => {
    if (classificationPreset === 'thermal_confirmed') return 'Throttling térmico · bajo la frecuencia garantizada';
    if (classificationPreset === 'mixed_limit') return 'Térmico + eléctrico';
    return 'Normal';
  });
  let headline = $derived.by(() => {
    if (classificationPreset === 'normal') return 'No se detectó throttling durante esta sesión.';
    return 'El calor redujo la frecuencia por debajo de la garantizada durante la carga sostenida; enfriar mejor devolvería un 10–20 %.';
  });

  // Before/after = two comparable guided tests (same profile, same power
  // context, same generator version), compared by MEASURED throughput.
  const comparisonMetrics: ReportComparisonMetric[] = [
    { label: 'Rendimiento sostenido de la carga de prueba', beforeValue: '78 %', afterValue: '93 %' },
    { label: 'Frecuencia activa sostenida (P)', beforeValue: '2,1 GHz', afterValue: '2,9 GHz' },
    { label: 'Temperatura media', beforeValue: '99 °C', afterValue: '88 °C' }
  ];
</script>

{#snippet perfDownIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M17 7 7 17M7 7v10h10" />
  </svg>
{/snippet}
{#snippet flameIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
    <path d="M8 3c0 4-4 5-4 10a6 6 0 0 0 12 0c0-2-1-3-2-4.5 0 2-1 3-2 3-1.5 0-2-1.5-1-3.5C10 6 9 4.5 8 3Z" />
  </svg>
{/snippet}
{#snippet clockIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" />
  </svg>
{/snippet}
{#snippet loadIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
    <path d="M4 17l5-6 4 3 5-8 4 5" />
  </svg>
{/snippet}

{#snippet exportActions()}
  <Button variant="secondary" label="Exportar…" onclick={() => (lastAction = 'abrir ExportDialog (informe)')} />
  <Button variant="secondary" label="Usar como referencia" onclick={() => (lastAction = 'usar como referencia')} />
{/snippet}

<div class="tw-ds report-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">CLASIFICACIÓN</span>
      <SegmentedControl
        label="Clasificación"
        value={classificationPreset}
        onchange={(v) => (classificationPreset = v as ClassificationPreset)}
        options={[
          { value: 'thermal_confirmed', label: 'Térmico' },
          { value: 'mixed_limit', label: 'Mixto' },
          { value: 'normal', label: 'Normal' }
        ]}
      />
    </div>
    <div class="control">
      <Switch bind:checked={loading} label="Cargando" />
    </div>
    <div class="control">
      <Switch bind:checked={sessionIncomplete} label="Sesión incompleta" />
    </div>
    <div class="control">
      <Switch bind:checked={reducedConfidence} label="Confianza reducida" />
    </div>
    <div class="control">
      <Switch bind:checked={provisional} label="Sesión en curso (provisional)" />
    </div>
    <div class="control">
      <Switch bind:checked={showReevaluated} label="Importada: reevaluación" />
    </div>
    <div class="control">
      <Switch bind:checked={hasBaseline} label="Potencial cuantificable (nivel A)" />
    </div>
    <div class="control">
      <Switch bind:checked={showCausalChain} label="Mostrar cadena causal" />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <ReportScreen
      status={loading ? 'loading' : 'ready'}
      loadingLabel="Generando informe…"
      {classification}
      {classificationLabel}
      {headline}
      {sessionIncomplete}
      incompleteNoticeTitle="Sesión incompleta"
      incompleteNoticeDescription="El diagnóstico se detuvo antes de completar la fase de recuperación — algunas conclusiones pueden ser menos precisas."
      {provisional}
      provisionalNoticeTitle="Provisional · sesión en curso"
      provisionalNoticeDescription="Este informe se recalcula con cada muestra y se congelará al cerrar la sesión."
      reevaluated={showReevaluated
        ? {
            title: 'Reevaluación con las reglas actuales (v1.2)',
            classification: 'thermal_probable',
            classificationLabel: 'Evidencia compatible con limitación térmica',
            headline: 'Con las reglas actuales la señal directa no alcanza la persistencia exigida.',
            note: 'El informe original (v1.0) se conserva arriba; la referencia del fichero importado no se aplica a este equipo.'
          }
        : undefined}
      {reducedConfidence}
      reducedConfidenceNoticeTitle="Confianza reducida"
      reducedConfidenceNoticeDescription="Uno o más sensores reportaron datos parciales durante esta sesión."
      observedTitle="Qué se observó"
      observedText="Tras el fin del turbo inicial (esperado), la temperatura se quedó clavada en el límite de 100 °C y el procesador declaró la razón térmica en el 73 % de las muestras, con la potencia por debajo de su límite. La frecuencia activa de los núcleos P bajó a 2,1 GHz, por debajo de su base de 2,6 GHz."
      causalChain={showCausalChain
        ? [
            { id: 'load', label: 'CARGA', value: '92 %', tone: 'accent', icon: loadIcon },
            { id: 'temp', label: 'TEMPERATURA', value: 'En el límite', tone: 'thermal', icon: flameIcon },
            { id: 'power', label: 'POTENCIA', value: '31 de 45 W', tone: 'accent', icon: perfDownIcon },
            { id: 'clock', label: 'FRECUENCIA', value: '2,1 < base 2,6', tone: 'thermal', icon: clockIcon }
          ]
        : undefined}
      impactTitle="Enfriar mejor"
      impactValue={hasBaseline ? '+10–20 % · mejora notable' : undefined}
      impactUnavailableTitle={hasBaseline ? undefined : 'No cuantificable en este equipo'}
      impactUnavailableReason={hasBaseline ? undefined : 'Hace falta leer el límite de potencia del procesador (acceso avanzado). Por la gravedad observada, la mejora probablemente sería notable.'}
      evidenceTitle="Evidencias"
      evidence={[
        'Razón THERMAL declarada por el procesador en el 73 % de las muestras de la fase sostenida.',
        'Temperatura en meseta a 99–100 °C (límite efectivo 100 °C) durante 4 minutos.',
        'Frecuencia activa de los núcleos P por debajo de la base en el 85 % de las muestras.',
        'Potencia a 31 W, por debajo de su límite de 45 W: no es una limitación de potencia.'
      ]}
      alternativeCausesTitle="Causas alternativas consideradas"
      alternativeCauses={[
        'Fin del turbo de potencia: registrado a los 42 s y excluido del análisis (es el comportamiento previsto).',
        'Gestión térmica del fabricante: descartada, el límite de potencia no cambió durante la sesión.',
        'Gestión de energía de Windows (EcoQoS, plan): descartada, el plan no cambió y la carga es de primer plano.'
      ]}
      cannotConcludeTitle="Qué no puede concluirse"
      cannotConclude={[
        'El potencial es una estimación física (límite de potencia frente a potencia medida); cargas limitadas por memoria mejorarían menos.',
        'No se puede afirmar cuál de las medidas de refrigeración es la causa (pasta, ventilador, obstrucción).'
      ]}
      recommendationsTitle="Recomendaciones"
      recommendations={[
        'Revisar la pasta térmica o el sistema de refrigeración si este patrón se repite en cargas similares.',
        'Repetir el diagnóstico guiado en un entorno con temperatura ambiente más baja para comparar.'
      ]}
      methodTitle="Cómo se ha calculado"
      methodDescription={hasBaseline ? 'Techo de potencia: límite PL1 45 W frente a 31 W medidos con la temperatura en el límite; ganancia de frecuencia (45/31)^⅓ − 1 ≈ 13 %, acotada por la frecuencia del turbo inicial; rango 0,5–1,0 × y redondeado a múltiplos de 5 %. Nivel de cobertura A.' : undefined}
      methodMissingText={hasBaseline ? undefined : 'Sin límite de potencia conocido no hay cifra. Nivel de cobertura B.'}
      comparisonTitle="Comparación antes / después"
      comparisonBeforeLabel="Antes"
      comparisonAfterLabel="Después"
      {comparisonMetrics}
      {exportActions}
    />
  </div>
</div>

<style>
  .report-demo {
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
    .report-demo {
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
