<script lang="ts">
  /**
   * Reference composition — NOT a component this system exports.
   * Shows how the pieces fit into one working "Ahora" screen: shell
   * (TitleBar + sidebar/BottomBar swap) + StatusHero + StatWidget grid
   * + CausalRail, wired to the width tracker so the whole thing
   * actually reflows the way the README's breakpoint table describes.
   *
   * Copy the pattern, not necessarily this exact file, into your app.
   * All strings here are hardcoded Spanish for brevity — a real app
   * pulls them from its ES/EN catalog.
   */
  import {
    TitleBar,
    NavigationItem,
    BottomBar,
    ToolbarButton,
    StatusHero,
    StatWidget,
    CausalRail,
    NavIcon,
    createWidthTracker
  } from '../components';
  import type { BottomBarItem, CausalNode, NavIconKind } from '../components';

  const win = createWidthTracker();

  let maximized = $state(false);

  // The full sidebar (>=980px shows all six with labels). BottomBar
  // below only ever surfaces 4 of these, per its own non-negotiable
  // rule: Ahora first, always.
  const navItems: { id: string; label: string; icon: NavIconKind }[] = [
    { id: 'now', label: 'Ahora', icon: 'now' },
    { id: 'analysis', label: 'Análisis', icon: 'analysis' },
    { id: 'cpu', label: 'CPU', icon: 'cpu' },
    { id: 'sessions', label: 'Sesiones', icon: 'sessions' },
    { id: 'guided', label: 'Diagnóstico guiado', icon: 'guided' },
    { id: 'settings', label: 'Ajustes', icon: 'settings' }
  ];
  const compactNavItems = navItems.filter((i) => ['now', 'analysis', 'cpu', 'guided'].includes(i.id));
  let active = $state('now');
</script>

{#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
{#snippet analysisIcon()}<NavIcon kind="analysis" />{/snippet}
{#snippet cpuIcon()}<NavIcon kind="cpu" />{/snippet}
{#snippet sessionsIcon()}<NavIcon kind="sessions" />{/snippet}
{#snippet guidedIcon()}<NavIcon kind="guided" />{/snippet}
{#snippet settingsIcon()}<NavIcon kind="settings" />{/snippet}

{#snippet coverageIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" /><path d="M12 8h.01M11 12h1v5h1" />
  </svg>
{/snippet}
{#snippet flameIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
    <path d="M8 3c0 4-4 5-4 10a6 6 0 0 0 12 0c0-2-1-3-2-4.5 0 2-1 3-2 3-1.5 0-2-1.5-1-3.5C10 6 9 4.5 8 3Z" />
  </svg>
{/snippet}
{#snippet loadIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
    <path d="M4 17l5-6 4 3 5-8 4 5" />
  </svg>
{/snippet}
{#snippet clockIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" />
  </svg>
{/snippet}
{#snippet boltIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
    <path d="M13 2 4 14h6l-1 8 9-12h-6z" />
  </svg>
{/snippet}
{#snippet perfDownIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M17 7 7 17M7 7v10h10" />
  </svg>
{/snippet}

<div class="tw-ds app-shell">
  <TitleBar
    title="ThrottleWatch"
    {maximized}
    onMinimize={() => {}}
    onMaximizeToggle={() => (maximized = !maximized)}
    onClose={() => {}}
    minimizeLabel="Minimizar"
    maximizeLabel="Maximizar"
    restoreLabel="Restaurar"
    closeLabel="Cerrar"
  />

  <div class="body" class:has-sidebar={win.tier !== 'compact'}>
    {#if win.tier !== 'compact'}
      <nav class="sidebar">
        {#each navItems as item (item.id)}
          <NavigationItem
            icon={item.id === 'now'
              ? nowIcon
              : item.id === 'analysis'
                ? analysisIcon
                : item.id === 'cpu'
                  ? cpuIcon
                  : item.id === 'sessions'
                    ? sessionsIcon
                    : item.id === 'guided'
                      ? guidedIcon
                      : settingsIcon}
            label={item.label}
            active={active === item.id}
            density={win.tier === 'expanded' ? 'labeled' : 'icon-only'}
            onclick={() => (active = item.id)}
          />
        {/each}
      </nav>
    {/if}

    <main class="content" style:padding={win.tier === 'compact' ? 'var(--space-4)' : win.tier === 'medium' ? 'var(--space-5)' : 'var(--space-6)'}>
      <header class="page-header">
        <h1 class="value-md" style:color="var(--text-primary)" style:margin="0">Ahora</h1>
        <ToolbarButton icon={coverageIcon} label="Ver cobertura" compact={win.tier === 'compact'} />
      </header>

      <StatusHero
        ringValue="98°"
        ringCaption="TjMax 100°"
        ringPercent={93}
        classification="thermal_confirmed"
        classificationLabel="LIMITACIÓN TÉRMICA CONFIRMADA"
        evidenceLine="4 °C hasta el límite · confianza alta · observado 3 min 42 s"
        performance={{ label: 'Enfriar mejor', rangeText: '+10–20 %' }}
        severity="below_base"
      />

      <div class="stat-grid">
        <StatWidget enterIndex={0} icon={flameIcon} tone="thermal" label="Temperatura" value="98" unit="°C" footnote="Margen −4 °C · directo" />
        <StatWidget enterIndex={1} icon={loadIcon} tone="accent" label="Carga" value="92" unit="%" footnote="P 95 % · E 88 %" />
        <StatWidget enterIndex={2} icon={clockIcon} tone="warm" label="Frecuencia activa" value="2.1" unit="GHz" footnote="Base 2,6 GHz · P 2,1 · E 1,8" />
        <StatWidget enterIndex={3} icon={boltIcon} tone="accent" label="Potencia" value="142" unit="W" footnote="Sin límite eléctrico activo" />
      </div>

      {#if win.tier === 'expanded'}
        <CausalRail
          nodes={[
            { id: 'load', label: 'CARGA', value: '92 %', tone: 'accent', icon: loadIcon },
            { id: 'temp', label: 'TEMPERATURA', value: 'Límite', tone: 'thermal', icon: flameIcon },
            { id: 'clock', label: 'FRECUENCIA', value: '2,1 < base 2,6', tone: 'thermal', icon: clockIcon },
            { id: 'perf', label: 'RENDIMIENTO', value: 'Menor', tone: 'thermal', icon: perfDownIcon }
          ] satisfies CausalNode[]}
        />
      {/if}
    </main>
  </div>

  {#if win.tier === 'compact'}
    <BottomBar
      items={compactNavItems.map((item) => ({
        id: item.id,
        label: item.label,
        icon:
          item.id === 'now'
            ? nowIcon
            : item.id === 'analysis'
              ? analysisIcon
              : item.id === 'cpu'
                ? cpuIcon
                : guidedIcon,
        active: active === item.id,
        onclick: () => (active = item.id)
      })) satisfies BottomBarItem[]}
    />
  {/if}
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3);
    border-right: 1px solid var(--hairline);
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
    gap: var(--space-5);
  }
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--space-2);
  }
</style>
