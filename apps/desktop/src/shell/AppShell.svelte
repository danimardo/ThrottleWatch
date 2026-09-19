<script lang="ts">
  import {
    BottomBar,
    EmptyState,
    NavigationItem,
    NavIcon,
    TitleBar,
    createWidthTracker,
    type NavIconKind
  } from '../design-system/components';
  import Dashboard from '../features/dashboard/Dashboard.svelte';
  import { createTranslator } from '../lib/i18n';

  type Destination = {
    id: 'now' | 'analysis' | 'cpu' | 'sessions' | 'guided' | 'settings';
    icon: NavIconKind;
    label: string;
  };

  const win = createWidthTracker();
  const { t } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );
  const destinations: Destination[] = [
    { id: 'now', icon: 'now', label: t('nav.now') },
    { id: 'analysis', icon: 'analysis', label: t('nav.analysis') },
    { id: 'cpu', icon: 'cpu', label: t('nav.cpu') },
    { id: 'sessions', icon: 'sessions', label: t('nav.sessions') },
    { id: 'guided', icon: 'guided', label: t('nav.guided') },
    { id: 'settings', icon: 'settings', label: t('nav.settings') }
  ];

  let active = $state<Destination['id']>('now');
  let maximized = $state(false);
  const compactDestinations = destinations.slice(0, 3);
  const overflowDestinations = destinations.slice(3);

  function select(id: Destination['id']): void {
    active = id;
  }
</script>

{#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
{#snippet analysisIcon()}<NavIcon kind="analysis" />{/snippet}
{#snippet cpuIcon()}<NavIcon kind="cpu" />{/snippet}
{#snippet sessionsIcon()}<NavIcon kind="sessions" />{/snippet}
{#snippet guidedIcon()}<NavIcon kind="guided" />{/snippet}
{#snippet settingsIcon()}<NavIcon kind="settings" />{/snippet}

<div class="tw-ds app-shell">
  <TitleBar
    title={t('app.title')}
    {maximized}
    onMinimize={() => undefined}
    onMaximizeToggle={() => (maximized = !maximized)}
    onClose={() => undefined}
    minimizeLabel={t('app.minimize')}
    maximizeLabel={t('app.maximize')}
    restoreLabel={t('app.restore')}
    closeLabel={t('app.close')}
  />
  <h1 class="visually-hidden">{t('app.title')}</h1>

  <div class="body" class:has-sidebar={win.tier !== 'compact'}>
    {#if win.tier !== 'compact'}
      <nav class="sidebar" aria-label={t('nav.moreDestinations')}>
        {#each destinations as item (item.id)}
          <NavigationItem
            icon={item.icon === 'now'
              ? nowIcon
              : item.icon === 'analysis'
                ? analysisIcon
                : item.icon === 'cpu'
                  ? cpuIcon
                  : item.icon === 'sessions'
                    ? sessionsIcon
                    : item.icon === 'guided'
                      ? guidedIcon
                      : settingsIcon}
            label={item.label}
            active={active === item.id}
            density={win.tier === 'expanded' ? 'labeled' : 'icon-only'}
            onclick={() => select(item.id)}
          />
        {/each}
      </nav>
    {/if}

    <div class="content">
      {#if active === 'now'}
        <Dashboard />
      {:else}
        <EmptyState
          icon={active === 'analysis'
            ? analysisIcon
            : active === 'cpu'
              ? cpuIcon
              : active === 'sessions'
                ? sessionsIcon
                : active === 'guided'
                  ? guidedIcon
                  : active === 'settings'
                    ? settingsIcon
                    : nowIcon}
          title={t('screens.comingSoon')}
          description={t('screens.notAvailable')}
        />
      {/if}
    </div>
  </div>

  {#if win.tier === 'compact'}
    <BottomBar
      items={compactDestinations.map((item) => ({
        id: item.id,
        label: item.label,
        icon:
          item.icon === 'now'
            ? nowIcon
            : item.icon === 'analysis'
              ? analysisIcon
              : cpuIcon,
        active: active === item.id,
        onclick: () => select(item.id)
      }))}
      menu={{
        id: 'more',
        label: t('nav.more'),
        menuLabel: t('nav.moreDestinations'),
        icon: settingsIcon,
        items: overflowDestinations.map((item) => ({
          id: item.id,
          label: item.label,
          icon:
            item.icon === 'sessions'
              ? sessionsIcon
              : item.icon === 'guided'
                ? guidedIcon
                : settingsIcon,
          active: active === item.id,
          onclick: () => select(item.id)
        }))
      }}
    />
  {/if}
</div>

<style>
  .app-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .body {
    min-height: 0;
    flex: 1;
    display: flex;
  }
  .sidebar {
    width: 196px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4) var(--space-3);
    border-right: 1px solid var(--hairline);
    background: var(--glass-bg-subtle);
  }
  .content {
    min-width: 0;
    flex: 1;
  }
  @media (max-width: 979px) {
    .sidebar {
      width: 48px;
      padding-inline: var(--space-2);
    }
  }
</style>
