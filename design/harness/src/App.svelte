<script lang="ts">
  /**
   * The harness's own index — browses every `*.example.svelte` in
   * ../examples/ with a visible nav and a theme toggle, so a person can
   * actually click through the package instead of hand-editing a URL.
   * `?view=<id>` is still read on load (and kept in sync) so scripted
   * screenshot tools can deep-link the same way the verification
   * scripts for this package always have.
   */
  import AhoraScreen from '../../examples/AhoraScreen.example.svelte';
  import SettingsSection from '../../examples/SettingsSection.example.svelte';
  import SettingsScreen from '../../examples/SettingsScreen.example.svelte';
  import OnboardingIllustrations from '../../examples/OnboardingIllustrations.example.svelte';
  import OnboardingFlow from '../../examples/OnboardingFlow.example.svelte';
  import MorePrimitives from '../../examples/MorePrimitives.example.svelte';
  import SessionsScreen from '../../examples/SessionsScreen.example.svelte';
  import GuidedDiagnosticScreen from '../../examples/GuidedDiagnosticScreen.example.svelte';
  import CpuScreen from '../../examples/CpuScreen.example.svelte';
  import AnalysisScreen from '../../examples/AnalysisScreen.example.svelte';
  import ReportScreen from '../../examples/ReportScreen.example.svelte';
  import ShellPieces from '../../examples/ShellPieces.example.svelte';
  import { applyGlassLevel, applyMotionLevel, type GlassLevel, type MotionLevel } from '../../tokens/tokens';

  const views = [
    { id: 'ahora', label: 'Ahora (pantalla completa)', component: AhoraScreen },
    { id: 'settings-primitives', label: 'Ajustes (recorte de primitivos)', component: SettingsSection },
    { id: 'settings', label: 'Ajustes (pantalla completa)', component: SettingsScreen },
    { id: 'sessions', label: 'Sesiones (pantalla completa)', component: SessionsScreen },
    { id: 'guided', label: 'Diagnóstico guiado (pantalla completa)', component: GuidedDiagnosticScreen },
    { id: 'cpu', label: 'CPU (pantalla completa)', component: CpuScreen },
    { id: 'analysis', label: 'Análisis (pantalla completa)', component: AnalysisScreen },
    { id: 'report', label: 'Informe (pantalla completa)', component: ReportScreen },
    { id: 'shell', label: 'Shell — cobertura, franja, diálogos, novedades, licencias, «Más»', component: ShellPieces },
    { id: 'onboarding', label: 'Onboarding — ilustraciones', component: OnboardingIllustrations },
    { id: 'onboarding-flow', label: 'Onboarding — flujo completo', component: OnboardingFlow },
    { id: 'more', label: 'Más primitivos (Tooltip/Select/ProgressBar/Banner/EmptyState)', component: MorePrimitives }
  ];

  const params = new URLSearchParams(location.search);
  let currentId = $state(params.get('view') ?? views[0].id);
  let theme: 'dark' | 'light' = $state('dark');
  let glass: GlassLevel = $state('full');
  let motion: MotionLevel = $state('system');

  let current = $derived(views.find((v) => v.id === currentId) ?? views[0]);
  let CurrentView = $derived(current.component);

  function go(id: string) {
    currentId = id;
    const url = new URL(location.href);
    url.searchParams.set('view', id);
    history.replaceState(null, '', url);
  }

  $effect(() => {
    document.documentElement.dataset.theme = theme;
  });
  $effect(() => applyGlassLevel(glass));
  $effect(() => applyMotionLevel(motion));
</script>

<div class="harness tw-ambient">
  <nav class="harness-nav">
    <span class="brand">ThrottleWatch — harness</span>
    <div class="links">
      {#each views as v (v.id)}
        <button type="button" class="nav-link" class:active={v.id === currentId} onclick={() => go(v.id)}>
          {v.label}
        </button>
      {/each}
    </div>
    <button type="button" class="theme-toggle" onclick={() => (theme = theme === 'dark' ? 'light' : 'dark')}>
      {theme === 'dark' ? '☾ Oscuro' : '☀ Claro'}
    </button>
    <label class="toggle">Vidrio
      <select bind:value={glass}>
        <option value="full">completo</option>
        <option value="reduced">reducido</option>
        <option value="off">sin</option>
      </select>
    </label>
    <label class="toggle">Movimiento
      <select bind:value={motion}>
        <option value="system">sistema</option>
        <option value="reduced">reducido</option>
        <option value="full">completo</option>
      </select>
    </label>
  </nav>

  <main class="harness-view">
    <CurrentView />
  </main>
</div>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
  }
  .harness {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toggle {
    font-size: 12px;
    color: var(--text-secondary);
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }
  .toggle select {
    font: inherit;
    background: var(--surface-raised);
    color: var(--text-primary);
    border: 1px solid var(--hairline);
    border-radius: 6px;
    padding: 2px 6px;
  }
  .harness-nav {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 16px;
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border-bottom: 1px solid var(--glass-border);
    position: sticky;
    top: 0;
    z-index: 10;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    flex-wrap: wrap;
  }
  .brand {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    flex: 0 0 auto;
  }
  .links {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    flex: 1;
  }
  .nav-link {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 650;
    padding: 6px 10px;
    border-radius: 8px;
    cursor: default;
  }
  .nav-link:hover {
    background: var(--surface-sunken);
  }
  .nav-link.active {
    background: color-mix(in srgb, var(--accent-blue) 14%, transparent);
    color: var(--accent-blue);
  }
  .nav-link:focus-visible,
  .theme-toggle:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .theme-toggle {
    flex: 0 0 auto;
    border: 1px solid var(--hairline);
    background: var(--surface-raised);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 650;
    padding: 6px 12px;
    border-radius: 8px;
  }
  .harness-view {
    flex: 1;
  }
</style>
