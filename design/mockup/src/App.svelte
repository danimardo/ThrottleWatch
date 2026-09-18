<script lang="ts">
  import {
    BottomBar,
    Banner,
    Button,
    CausalRail,
    ContextStrip,
    CoverageMatrix,
    ExportDialog,
    FirstCloseDialog,
    NavigationItem,
    NavIcon,
    OnboardingFlow,
    StatWidget,
    StatusHero,
    TitleBar,
    ToolbarButton,
    WhatsNewCards,
    createWidthTracker
  } from '../../components';
  import { applyGlassLevel, applyMotionLevel, motionIsReduced, type GlassLevel, type MotionLevel } from '../../tokens/tokens';
  import { fly, fade } from 'svelte/transition';
  import type {
    BottomBarItem,
    CausalNode,
    NavIconKind,
    CollectorState,
    CoverageRow,
    ExportFormat,
    WhatsNewCard,
    AdvancedAccessState
  } from '../../components';
  import AnalysisDemo from '../../examples/AnalysisScreen.example.svelte';
  import CpuDemo from '../../examples/CpuScreen.example.svelte';
  import GuidedDemo from '../../examples/GuidedDiagnosticScreen.example.svelte';
  import SessionsDemo from '../../examples/SessionsScreen.example.svelte';
  import SettingsDemo from '../../examples/SettingsScreen.example.svelte';
  import ReportDemo from '../../examples/ReportScreen.example.svelte';

  type View = 'now' | 'analysis' | 'cpu' | 'sessions' | 'guided' | 'settings' | 'report';

  const win = createWidthTracker();
  let active: View = $state('now');
  let theme: 'dark' | 'light' = $state('dark');
  let maximized = $state(false);
  let showOnboarding = $state(true);
  let showDemoNote = $state(true);
  let glass: GlassLevel = $state('full');
  let motion: MotionLevel = $state('system');
  $effect(() => applyGlassLevel(glass));
  $effect(() => applyMotionLevel(motion));
  // Screen transition parameters: instant when motion is reduced.
  const screenIn = () => (motionIsReduced() ? { duration: 0 } : { y: 14, duration: 320, easing: (t: number) => 1 - Math.pow(1 - t, 3) });
  const screenOut = () => (motionIsReduced() ? { duration: 0 } : { duration: 120 });

  // --- Estados simulados del shell (el mockup no tiene backend) ---
  let collectorState: CollectorState = $state('fresh');
  let guidedRunning = $state(false);
  let coverageOpen = $state(false);
  let exportOpen = $state(false);
  let exportFormat: ExportFormat = $state('csv');
  let exportAnonymize = $state(true);
  let closeAction: 'unset' | 'exit' | 'tray' = $state('unset');
  let firstCloseOpen = $state(false);
  let closeFeedback = $state('');
  const advancedAccess: AdvancedAccessState = 'installable';

  let whatsNew: WhatsNewCard[] = $state([
    {
      id: 'n1',
      title: 'El acceso avanzado se instala por separado',
      body: 'ThrottleWatch ya no pide permisos de administrador al abrirse. Si quieres banderas térmicas directas, instálalo desde Ajustes → Sensores.',
      actionLabel: 'Ir a Ajustes',
      onAction: () => go('settings')
    }
  ]);

  const navItems: { id: Exclude<View, 'report'>; label: string; icon: NavIconKind }[] = [
    { id: 'now', label: 'Ahora', icon: 'now' },
    { id: 'analysis', label: 'Análisis', icon: 'analysis' },
    { id: 'cpu', label: 'CPU', icon: 'cpu' },
    { id: 'sessions', label: 'Sesiones', icon: 'sessions' },
    { id: 'guided', label: 'Diagnóstico guiado', icon: 'guided' },
    { id: 'settings', label: 'Ajustes', icon: 'settings' }
  ];

  const pageTitles: Record<View, string> = {
    now: 'Ahora',
    analysis: 'Análisis',
    cpu: 'CPU',
    sessions: 'Sesiones',
    guided: 'Diagnóstico guiado',
    settings: 'Ajustes',
    report: 'Informe de la sesión'
  };

  // Simulated live samples so the value-change bump and the ring
  // interpolation are visible in the mockup (1 Hz, tiny jitter).
  let liveTemp = $state('98');
  let liveLoad = $state('92');
  let livePower = $state('64');
  let liveRing = $state(93);
  $effect(() => {
    if (showOnboarding) return;
    const id = setInterval(() => {
      if (collectorState !== 'fresh') return;
      const t = 96 + Math.round(Math.random() * 3);
      liveTemp = String(t);
      liveRing = t - 5;
      liveLoad = String(88 + Math.round(Math.random() * 8));
      livePower = String(60 + Math.round(Math.random() * 9));
    }, 1000);
    return () => clearInterval(id);
  });

  const COLLECTOR_LABELS: Record<CollectorState, string> = {
    fresh: 'Conectado · hace 1 s',
    stale: 'Datos obsoletos · hace 12 s',
    disconnected: 'Colector desconectado',
    starting: 'Iniciando colector…'
  };

  $effect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.lang = 'es';
  });

  $effect(() => {
    if (!showOnboarding) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && e.key >= '1' && e.key <= '6') {
        e.preventDefault();
        go(navItems[Number(e.key) - 1].id);
      } else if (e.ctrlKey && e.key === ',') {
        e.preventDefault();
        go('settings');
      } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'x' && guidedRunning) {
        e.preventDefault();
        guidedRunning = false;
      } else if (e.ctrlKey && e.key.toLowerCase() === 'e') {
        e.preventDefault();
        exportOpen = true;
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  const onboardingSteps: [
    { title: string; body: string },
    { title: string; body: string },
    { title: string; body: string },
    { title: string; body: string },
    {
      title: string;
      body: string;
      status: 'complete';
      detectingLabel: string;
      coverageTitle: string;
      coverageDescription: string;
      advancedAccess: AdvancedAccessState;
      advancedAccessNote: string;
      requestAccessLabel: string;
      onRequestAdvancedAccess: () => void;
      retryLabel: string;
    }
  ] = [
    {
      title: 'Entiende si el calor está limitando tu CPU',
      body: 'ThrottleWatch observa temperatura, carga, reloj y potencia para explicarte qué ocurre sin convertir una lectura caliente en una conclusión precipitada. Nunca modifica tu equipo.'
    },
    {
      title: 'Las señales se entienden mejor juntas',
      body: 'Una temperatura alta por sí sola no demuestra pérdida de rendimiento. Comparamos las señales durante el mismo intervalo. Thermal throttling es la reducción automática de velocidad para evitar demasiado calor.'
    },
    {
      title: 'Conclusiones claras y prudentes',
      body: 'Diferenciamos observaciones, indicios y limitación confirmada, mostrando siempre la evidencia y el nivel de confianza. Solo hay porcentaje cuando existe una referencia comparable de este mismo equipo.'
    },
    {
      title: 'Tus datos permanecen en este equipo',
      body: 'Todo funciona sin Internet y nada se envía fuera. Se conservan siete días de historial, los avisos están apagados y puedes cambiar idioma, tema y segundo plano desde Ajustes.'
    },
    {
      title: 'Comprobemos este equipo',
      body: 'La detección inicial es pasiva y no solicita permisos de administrador.',
      status: 'complete',
      detectingLabel: 'Detectando sensores…',
      coverageTitle: 'Cobertura parcial',
      coverageDescription: 'Temperatura, carga, reloj efectivo (derivado) y potencia disponibles. Sin bandera térmica directa: la confianza máxima será «probable».',
      advancedAccess,
      advancedAccessNote: 'Puedes instalar el acceso avanzado para obtener banderas térmicas directas. Requiere permisos de administrador una sola vez.',
      requestAccessLabel: 'Instalar acceso avanzado',
      onRequestAdvancedAccess: () => {},
      retryLabel: 'Reintentar'
    }
  ];

  const coverageRows: CoverageRow[] = [
    { id: 'temperature', label: 'Temperatura', available: true, quality: 'direct', qualityLabel: 'Directo', sourceLabel: 'CPU Package' },
    { id: 'headroom', label: 'Margen térmico', available: true, quality: 'derived', qualityLabel: 'Derivado', sourceLabel: 'TjMax 100 °C − paquete' },
    { id: 'load', label: 'Carga', available: true, quality: 'direct', qualityLabel: 'Directo', sourceLabel: 'CPU Total' },
    { id: 'clock', label: 'Reloj efectivo', available: true, quality: 'derived', qualityLabel: 'Derivado', sourceLabel: '% Processor Performance × base' },
    { id: 'power', label: 'Potencia', available: true, quality: 'direct', qualityLabel: 'Directo', sourceLabel: 'CPU Package Power' },
    { id: 'thermal_flag', label: 'Bandera térmica', available: false, reasonLabel: 'Requiere acceso avanzado.' },
    { id: 'power_flag', label: 'Bandera eléctrica', available: false, reasonLabel: 'Requiere acceso avanzado.' }
  ];

  let includedFields = $derived.by(() =>
    exportFormat === 'csv'
      ? ['timestamp_utc', 'monotonic_ms', 'sensor_id', 'metric', 'scope', 'value', 'status', 'quality', 'cpu.vendor', 'cpu.display_name', 'cpu.topology', 'versions']
      : ['classification', 'confidence_band', 'evidence', 'alternative_causes', 'events', 'baseline', 'ruleset_version', 'cpu.vendor', 'cpu.display_name', 'cpu.topology']
  );
  let excludedFields = $derived.by(() =>
    exportAnonymize
      ? ['hostname', 'usuario de Windows', 'números de serie', 'direcciones MAC', 'rutas locales', 'huella de monitores', 'GUID del plan de energía']
      : []
  );

  function go(view: View) {
    active = view;
    coverageOpen = false;
  }

  function requestClose() {
    if (closeAction === 'unset') {
      firstCloseOpen = true;
    } else {
      closeFeedback = closeAction === 'tray' ? 'Ventana oculta en la bandeja (simulado).' : 'ThrottleWatch se cerraría (simulado).';
    }
  }
</script>

{#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
{#snippet analysisIcon()}<NavIcon kind="analysis" />{/snippet}
{#snippet cpuIcon()}<NavIcon kind="cpu" />{/snippet}
{#snippet sessionsIcon()}<NavIcon kind="sessions" />{/snippet}
{#snippet guidedIcon()}<NavIcon kind="guided" />{/snippet}
{#snippet settingsIcon()}<NavIcon kind="settings" />{/snippet}
{#snippet moreIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="5" cy="12" r="1.4" fill="currentColor" /><circle cx="12" cy="12" r="1.4" fill="currentColor" /><circle cx="19" cy="12" r="1.4" fill="currentColor" /></svg>
{/snippet}
{#snippet themeIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <path d="M20 15.2A8.3 8.3 0 0 1 8.8 4 8.4 8.4 0 1 0 20 15.2Z" />
  </svg>
{/snippet}
{#snippet exportIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 4v11M8 11l4 4 4-4M4 19h16" />
  </svg>
{/snippet}
{#snippet flameIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M8 3c0 4-4 5-4 10a6 6 0 0 0 12 0c0-2-1-3-2-4.5 0 2-1 3-2 3-1.5 0-2-1.5-1-3.5C10 6 9 4.5 8 3Z" /></svg>
{/snippet}
{#snippet loadIcon()}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M4 17l5-6 4 3 5-8 4 5" /></svg>{/snippet}
{#snippet clockIcon()}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>{/snippet}
{#snippet boltIcon()}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M13 2 4 14h6l-1 8 9-12h-6z" /></svg>{/snippet}
{#snippet perfIcon()}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 7 7 17M7 7v10h10" /></svg>{/snippet}
{#snippet stopAction()}
  <Button variant="secondary" label="Detener" onclick={() => (guidedRunning = false)} />
{/snippet}
{#snippet collectorActions()}
  <div class="banner-actions">
    <Button variant="secondary" label="Reintentar" onclick={() => (collectorState = 'starting')} />
    <Button variant="secondary" label="Ver resumen técnico" onclick={() => go('settings')} />
  </div>
{/snippet}

{#if showOnboarding}
  <div class="onboarding-shell tw-ds tw-ambient">
    <TitleBar
      title="ThrottleWatch"
      {maximized}
      onMinimize={() => {}}
      onMaximizeToggle={() => (maximized = !maximized)}
      onClose={() => (showOnboarding = false)}
      minimizeLabel="Minimizar"
      maximizeLabel="Maximizar"
      restoreLabel="Restaurar"
      closeLabel="Cerrar"
    />
    <main class="onboarding-content">
      <OnboardingFlow
        steps={onboardingSteps}
        progressLabel={(step, count) => `Paso ${step} de ${count}`}
        backLabel="Atrás"
        nextLabel="Siguiente"
        skipLabel="Omitir"
        finishLabel="Abrir Ahora"
        resumedNote="Puedes continuar donde lo dejaste."
        onSkip={() => (showOnboarding = false)}
        onFinish={() => (showOnboarding = false)}
      />
    </main>
  </div>
{:else}
  <div class="app-shell tw-ds tw-ambient">
    <TitleBar
      title="ThrottleWatch"
      {maximized}
      onMinimize={() => {}}
      onMaximizeToggle={() => (maximized = !maximized)}
      onClose={requestClose}
      minimizeLabel="Minimizar"
      maximizeLabel="Maximizar"
      restoreLabel="Restaurar"
      closeLabel="Cerrar"
    />

    <!-- Barras globales bajo la barra de título: visibles en cualquier pantalla y ancho. -->
    {#if guidedRunning || collectorState === 'disconnected' || collectorState === 'starting'}
      <div class="global-notices">
        {#if guidedRunning}
          <Banner tone="warning" title="Prueba en curso · Carga sostenida · quedan 1 min 21 s" action={stopAction} />
        {/if}
        {#if collectorState === 'disconnected'}
          <Banner tone="critical" title="El colector de sensores se ha detenido" description="Reintentando (2 de 3). Los datos mostrados están obsoletos." action={collectorActions} />
        {:else if collectorState === 'starting'}
          <Banner tone="info" title="Reiniciando el colector de sensores…" />
        {/if}
      </div>
    {/if}

    <div class="app-body" class:has-sidebar={win.tier !== 'compact'}>
      {#if win.tier !== 'compact'}
        <nav class="sidebar" aria-label="Navegación principal">
          {#each navItems as item (item.id)}
            <NavigationItem
              icon={item.id === 'now' ? nowIcon : item.id === 'analysis' ? analysisIcon : item.id === 'cpu' ? cpuIcon : item.id === 'sessions' ? sessionsIcon : item.id === 'guided' ? guidedIcon : settingsIcon}
              label={item.label}
              active={active === item.id}
              density={win.tier === 'expanded' ? 'labeled' : 'icon-only'}
              onclick={() => go(item.id)}
            />
          {/each}
        </nav>
      {/if}

      <main class="content" style:padding={win.tier === 'compact' ? 'var(--space-4)' : win.tier === 'medium' ? 'var(--space-5)' : 'var(--space-6)'}>
        <header class="page-header">
          <div class="heading-group">
            {#if active === 'report'}
              <button class="back body-strong" type="button" onclick={() => go('sessions')}>← Sesiones</button>
            {/if}
            <h1 class="value-md">{pageTitles[active]}</h1>
          </div>
          <div class="header-actions">
            {#if active === 'now' || active === 'report' || active === 'analysis'}
              <ToolbarButton icon={exportIcon} label="Exportar…" compact={win.tier === 'compact'} onclick={() => (exportOpen = true)} />
            {/if}
            <ToolbarButton icon={themeIcon} label={theme === 'dark' ? 'Tema claro' : 'Tema oscuro'} compact={win.tier === 'compact'} onclick={() => (theme = theme === 'dark' ? 'light' : 'dark')} />
          </div>
        </header>

        {#if showDemoNote}
          <div class="mockup-note body">
            <span><strong>Mockup navegable:</strong> datos simulados, sin hardware. Simular:</span>
            <div class="mockup-controls">
              <button type="button" class="chip" class:on={guidedRunning} onclick={() => (guidedRunning = !guidedRunning)}>Prueba en curso</button>
              <button type="button" class="chip" class:on={collectorState === 'stale'} onclick={() => (collectorState = collectorState === 'stale' ? 'fresh' : 'stale')}>Datos obsoletos</button>
              <button type="button" class="chip" class:on={collectorState === 'disconnected'} onclick={() => (collectorState = collectorState === 'disconnected' ? 'fresh' : 'disconnected')}>Colector caído</button>
              <button type="button" class="chip" onclick={() => (closeAction = 'unset')}>Olvidar cierre</button>
              <span class="chip-sep" aria-hidden="true">·</span>
              <button type="button" class="chip" class:on={glass === 'full'} onclick={() => (glass = 'full')}>Vidrio</button>
              <button type="button" class="chip" class:on={glass === 'reduced'} onclick={() => (glass = 'reduced')}>Vidrio reducido</button>
              <button type="button" class="chip" class:on={glass === 'off'} onclick={() => (glass = 'off')}>Sin vidrio</button>
              <button type="button" class="chip" class:on={motion === 'reduced'} onclick={() => (motion = motion === 'reduced' ? 'system' : 'reduced')}>Movimiento reducido</button>
            </div>
            <button type="button" aria-label="Cerrar aviso" onclick={() => (showDemoNote = false)}>×</button>
          </div>
        {/if}
        {#if closeFeedback}
          <Banner tone="info" title={closeFeedback} onDismiss={() => (closeFeedback = '')} dismissLabel="Cerrar aviso" />
        {/if}

        {#key active}
        <section class="screen-host" aria-label={pageTitles[active]} in:fly={screenIn()} out:fade={screenOut()}>
          {#if active === 'now'}
            <WhatsNewCards
              title="Novedades de la versión 1.5"
              cards={whatsNew}
              dismissLabel="Entendido"
              onDismiss={(id) => (whatsNew = whatsNew.filter((c) => c.id !== id))}
            />
            <ContextStrip
              cpuLabel="Intel Core Ultra 7 155H"
              topologyLabel="6P + 8E + 2LP"
              powerLabel="CA · Equilibrado"
              {collectorState}
              collectorLabel={COLLECTOR_LABELS[collectorState]}
              coverageActionLabel="Ver cobertura"
              onCoverage={() => (coverageOpen = true)}
              compact={win.tier === 'compact'}
            />
            {#if collectorState === 'disconnected' || collectorState === 'starting'}
              <StatusHero
                ringValue="—"
                ringCaption="Sin datos"
                ringPercent={0}
                classification="indeterminate"
                classificationLabel="DATOS INSUFICIENTES"
                evidenceLine="El colector no está enviando muestras. La última lectura válida es de hace 12 s."
                noBaselineText="Sin muestras no se evalúa el rendimiento."
              />
            {:else}
              <StatusHero
                ringValue={liveTemp + '°'}
                ringCaption="TjMax 100°"
                ringPercent={liveRing}
                classification="thermal_probable"
                classificationLabel="EVIDENCIA COMPATIBLE CON LIMITACIÓN TÉRMICA"
                evidenceLine="2 °C hasta el límite · confianza media (sin bandera directa) · observado 3 min 42 s"
                performance={{ label: 'Rendimiento disponible estimado', rangeText: '76–86 %', percent: 81 }}
              />
            {/if}
            <div class="stat-grid">
              <StatWidget enterIndex={0} icon={flameIcon} tone={collectorState === 'stale' ? 'unknown' : 'thermal'} label="Temperatura" value={liveTemp} unit="°C" footnote="Margen 2 °C · directo (CPU Package)" />
              <StatWidget enterIndex={1} icon={loadIcon} tone="accent" label="Carga" value={liveLoad} unit="%" footnote="P 95 % · E 88 % · LP 40 %" />
              <StatWidget enterIndex={2} icon={clockIcon} tone="warm" label="Reloj efectivo" value="3.4" unit="GHz" footnote="P 3.4 · E 2.6 · derivado · caída sostenida" />
              <StatWidget enterIndex={3} icon={boltIcon} tone="accent" label="Potencia" value={livePower} unit="W" footnote="Sin límite eléctrico observado" />
            </div>
            {#if win.tier === 'expanded' && collectorState !== 'disconnected'}
              <CausalRail nodes={[
                { id: 'load', label: 'CARGA', value: '92 %', tone: 'accent', icon: loadIcon },
                { id: 'temp', label: 'TEMPERATURA', value: 'Límite', tone: 'thermal', icon: flameIcon },
                { id: 'clock', label: 'RELOJ', value: '3.4 GHz ↓', tone: 'thermal', icon: clockIcon },
                { id: 'perf', label: 'RENDIMIENTO', value: 'Menor', tone: 'thermal', icon: perfIcon }
              ] satisfies CausalNode[]} />
            {/if}
            <button class="report-link body-strong" type="button" onclick={() => go('report')}>Ver informe (provisional · sesión en curso) →</button>
          {:else if active === 'analysis'}
            <AnalysisDemo />
          {:else if active === 'cpu'}
            <CpuDemo />
          {:else if active === 'sessions'}
            <SessionsDemo />
          {:else if active === 'guided'}
            <GuidedDemo />
          {:else if active === 'settings'}
            <SettingsDemo />
          {:else}
            <ReportDemo />
          {/if}
        </section>
        {/key}
      </main>

      {#if coverageOpen}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="panel-scrim" onclick={() => (coverageOpen = false)} aria-hidden="true"></div>
        <aside class="coverage-panel" aria-label="Cobertura de este equipo">
          <div class="panel-head">
            <h2 class="value-md">Cobertura de este equipo</h2>
            <button type="button" class="panel-close" aria-label="Cerrar panel" onclick={() => (coverageOpen = false)}>×</button>
          </div>
          <CoverageMatrix
            rows={coverageRows}
            columnLabels={{ magnitude: 'Magnitud', available: 'Disponible', quality: 'Calidad', source: 'Origen', reason: 'Motivo' }}
            availableLabel="Sí"
            unavailableLabel="No"
            maxConfidenceLabel="Confianza máxima alcanzable en este equipo: probable (sin bandera térmica directa)"
            accessState={advancedAccess}
            accessLabel="Puedes instalar el acceso avanzado para obtener banderas térmicas y eléctricas directas. Requiere permisos de administrador una sola vez."
            requestAccessLabel="Instalar acceso avanzado"
            onRequestAccess={() => {}}
            recheckLabel="Volver a comprobar"
            onRecheck={() => (collectorState = 'starting')}
            copySummaryLabel="Copiar resumen técnico"
            onCopySummary={() => {}}
          />
        </aside>
      {/if}
    </div>

    {#if win.tier === 'compact'}
      <BottomBar
        items={navItems.filter((item) => ['now', 'analysis', 'cpu'].includes(item.id)).map((item) => ({
          id: item.id,
          label: item.label,
          icon: item.id === 'now' ? nowIcon : item.id === 'analysis' ? analysisIcon : cpuIcon,
          active: active === item.id,
          onclick: () => go(item.id)
        })) satisfies BottomBarItem[]}
        menu={{
          id: 'more',
          label: 'Más',
          icon: moreIcon,
          menuLabel: 'Más destinos',
          items: [
            { id: 'sessions', label: 'Sesiones', icon: sessionsIcon, active: active === 'sessions' || active === 'report', onclick: () => go('sessions') },
            { id: 'guided', label: 'Diagnóstico guiado', icon: guidedIcon, active: active === 'guided', onclick: () => go('guided') },
            { id: 'settings', label: 'Ajustes', icon: settingsIcon, active: active === 'settings', onclick: () => go('settings') }
          ]
        }}
      />
    {/if}
  </div>

  <FirstCloseDialog
    bind:open={firstCloseOpen}
    title="¿Qué quieres hacer al cerrar?"
    description="Puedes salir de ThrottleWatch o dejarla en la bandeja del sistema para que siga midiendo."
    trayNote="Elegir la bandeja activa la monitorización en segundo plano."
    settingsHint="Puedes cambiarlo cuando quieras en Ajustes → General."
    exitLabel="Salir de ThrottleWatch"
    trayLabel="Continuar en la bandeja y seguir midiendo"
    onExit={() => {
      firstCloseOpen = false;
      closeAction = 'exit';
      closeFeedback = 'Elegido «Salir». ThrottleWatch se cerraría (simulado).';
    }}
    onTray={() => {
      firstCloseOpen = false;
      closeAction = 'tray';
      closeFeedback = 'Elegido «Bandeja»: monitorización en segundo plano activada y ventana oculta (simulado).';
    }}
    onDismiss={() => {
      firstCloseOpen = false;
      closeFeedback = 'Diálogo cerrado sin decidir: no se guarda nada y la ventana sigue abierta.';
    }}
  />

  <ExportDialog
    bind:open={exportOpen}
    title={active === 'analysis' ? 'Exportar rango seleccionado' : active === 'report' ? 'Exportar informe' : 'Exportar sesión'}
    scopeLabel={active === 'analysis' ? 'Rango seleccionado · 02:10–05:40 · sesión en curso' : 'Sesión en curso · Monitorización continua · desde 14:32'}
    formatLabel="Formato"
    format={exportFormat}
    formatOptions={[
      { value: 'csv', label: 'CSV de muestras' },
      { value: 'json', label: 'JSON de informe' }
    ]}
    onFormatChange={(f) => (exportFormat = f)}
    anonymizeLabel="Anonimizar"
    anonymizeDescription="Elimina identificadores del equipo y del usuario. Nada sale del equipo por red."
    anonymize={exportAnonymize}
    onAnonymizeChange={(v) => (exportAnonymize = v)}
    includedTitle="Se incluye"
    {includedFields}
    excludedTitle="Se excluye"
    {excludedFields}
    sizeLabel={exportFormat === 'csv' ? 'Tamaño estimado: 1,8 MB' : 'Tamaño estimado: 42 KB'}
    fileNameLabel={exportFormat === 'csv' ? 'throttlewatch_session_2026-09-18_a1b2c3.csv' : 'throttlewatch_report_2026-09-18_a1b2c3.json'}
    cancelLabel="Cancelar"
    confirmLabel="Elegir carpeta y exportar"
    onCancel={() => (exportOpen = false)}
    onConfirm={() => (exportOpen = false)}
  />
{/if}

<style>
  :global(html), :global(body), :global(#app) {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  :global(button), :global(input) { font: inherit; }
  .app-shell, .onboarding-shell {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
  }
  .chip-sep { color: var(--text-tertiary); }
  .onboarding-content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    place-items: center;
    padding: var(--space-5);
  }
  .onboarding-content :global(.tw-onboarding-slide) { width: min(880px, 100%); }
  .global-notices {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) 0;
  }
  .banner-actions { display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .app-body { display: flex; flex: 1; min-height: 0; position: relative; }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3);
    flex: 0 0 auto;
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border-right: 1px solid var(--glass-border);
  }
  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .page-header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); }
  .heading-group, .header-actions { display: flex; align-items: center; gap: var(--space-3); }
  h1 { color: var(--text-primary); margin: 0; }
  .back, .report-link {
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--accent-blue);
    cursor: default;
  }
  .back:hover, .report-link:hover {
    text-decoration: none;
  }
  .mockup-note {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background-color: var(--glass-bg-subtle);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    flex-wrap: wrap;
  }
  .mockup-note > button { border: 0; background: transparent; color: var(--text-secondary); cursor: default; font-size: 20px; }
  .mockup-controls { display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .chip {
    border: 1px solid var(--hairline);
    background: transparent;
    color: var(--text-secondary);
    border-radius: var(--radius-full);
    padding: 2px 10px;
    font-size: 12px;
    cursor: default;
  }
  .chip.on { color: var(--accent-blue); border-color: var(--accent-blue); }
  .screen-host { display: flex; flex-direction: column; gap: var(--space-5); min-width: 0; }
  .stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: var(--space-2); }
  .report-link { align-self: flex-start; padding-block: var(--space-2); }

  .panel-scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.32);
    z-index: 5;
    animation: tw-fade var(--motion-base) ease-out both;
  }
  @keyframes tw-panel-in {
    from { transform: translateX(40px); opacity: 0; }
    to { transform: none; opacity: 1; }
  }
  .coverage-panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(680px, 100%);
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border-left: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    animation: tw-panel-in var(--motion-slow) var(--motion-ease-out) both;
    padding: var(--space-5);
    overflow: auto;
    z-index: 6;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .panel-head { display: flex; align-items: center; justify-content: space-between; }
  .panel-head h2 { margin: 0; color: var(--text-primary); }
  .panel-close {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font-size: 22px;
    cursor: default;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
  }
  .panel-close:focus-visible { outline: 2px solid var(--accent-blue); }

  /* Los controles técnicos pertenecen a los ejemplos, no a la aplicación final. */
  .screen-host :global(.control-panel) { display: none !important; }
  .screen-host :global(.screen-frame), .screen-host :global(.flow-frame) { padding: 0 !important; border: 0 !important; background: transparent !important; }
  .screen-host :global(.analysis-demo),
  .screen-host :global(.cpu-demo),
  .screen-host :global(.guided-demo),
  .screen-host :global(.sessions-demo),
  .screen-host :global(.settings-screen-demo),
  .screen-host :global(.report-demo) { padding: 0 !important; background: transparent !important; min-height: 0 !important; }

  @media (max-width: 699px) {
    .page-header { align-items: flex-start; }
    .heading-group { align-items: flex-start; flex-direction: column; gap: var(--space-1); }
    .mockup-note { align-items: flex-start; }
  }
</style>
