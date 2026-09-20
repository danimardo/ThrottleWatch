<script lang="ts">
  import { onMount } from 'svelte';
  import {
    BottomBar,
    EmptyState,
    NavigationItem,
    NavIcon,
    OnboardingFlow,
    TitleBar,
    WhatsNewCards,
    createWidthTracker,
    type NavIconKind,
    type OnboardingDetectionContent,
    type OnboardingStepContent
  } from '../design-system/components';
  import Dashboard from '../features/dashboard/Dashboard.svelte';
  import {
    commandResponseSchemas,
    type CoverageMatrix,
    type OnboardingState
  } from '../lib/bridge/schemas';
  import { invokeValidated } from '../lib/bridge';
  import { createTranslator } from '../lib/i18n';
  import {
    createWindowAdapter,
    type WindowAdapter
  } from '../lib/bridge/window';
  import {
    advanceToSlide,
    CURRENT_NOTICE_VERSION,
    initialStepFor,
    markNoticesSeen,
    resolveOnboarding
  } from '../features/onboarding/model';
  import {
    loadOnboardingState,
    saveOnboardingState
  } from '../features/onboarding/repository';
  import {
    applyAppearance,
    listenToSystemAppearance
  } from '../features/appearance/appearance';

  type Destination = {
    id: 'now' | 'analysis' | 'cpu' | 'sessions' | 'guided' | 'settings';
    icon: NavIconKind;
    label: string;
  };

  const win = createWidthTracker();
  const { t, locale } = createTranslator(
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
  let windowAdapter: WindowAdapter = createWindowAdapter();
  let onboardingState = $state<OnboardingState | null>(null);
  let detectionStatus =
    $state<OnboardingDetectionContent['status']>('detecting');
  let coverage = $state<CoverageMatrix | null>(null);
  let detectionStarted = false;
  const compactDestinations = destinations.slice(0, 3);
  const overflowDestinations = destinations.slice(3);

  let onboardingActive = $derived(onboardingState?.status === 'pending');
  let noticesVisible = $derived(
    onboardingState !== null &&
      !onboardingActive &&
      onboardingState.last_seen_notice_version < CURRENT_NOTICE_VERSION
  );

  onMount(() => {
    windowAdapter = createWindowAdapter();
    void windowAdapter.isMaximized().then((value) => (maximized = value));
    void windowAdapter.restoreSavedGeometry();
    let stopGeometryListener: () => void = () => undefined;
    void windowAdapter
      .watchGeometryChanges(() => void windowAdapter.persistGeometry())
      .then((stop) => (stopGeometryListener = stop));
    applyAppearance('system', 'system', locale);
    const stopAppearanceListener = listenToSystemAppearance('system', () => {
      applyAppearance('system', 'system', locale);
    });
    void loadOnboardingState().then((state) => {
      onboardingState = state;
      if (state.status === 'pending' && initialStepFor(state) === 4) {
        void startDetection();
      }
    });
    return () => {
      stopAppearanceListener();
      stopGeometryListener();
    };
  });

  function select(id: Destination['id']): void {
    active = id;
  }

  function onboardingSteps(): [
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingStepContent,
    OnboardingDetectionContent
  ] {
    const detection: OnboardingDetectionContent = {
      title: t('onboarding.slide5.title'),
      body: t('onboarding.slide5.body'),
      status: detectionStatus,
      detectingLabel: t('onboarding.detecting'),
      coverageTitle:
        coverage?.tier === 'A'
          ? t('onboarding.coverage.completeTitle')
          : t('onboarding.coverage.partialTitle'),
      coverageDescription:
        coverage?.tier === 'A'
          ? t('onboarding.coverage.completeDescription')
          : t('onboarding.coverage.partialDescription'),
      advancedAccess: coverage?.advanced_access,
      advancedAccessNote:
        coverage?.advanced_access === 'installable'
          ? t('onboarding.access.installable')
          : coverage?.advanced_access === 'available'
            ? t('onboarding.access.available')
            : coverage?.advanced_access === 'denied'
              ? t('onboarding.access.denied')
              : undefined
    };
    return [
      {
        title: t('onboarding.slide1.title'),
        body: t('onboarding.slide1.body')
      },
      {
        title: t('onboarding.slide2.title'),
        body: t('onboarding.slide2.body')
      },
      {
        title: t('onboarding.slide3.title'),
        body: t('onboarding.slide3.body')
      },
      {
        title: t('onboarding.slide4.title'),
        body: t('onboarding.slide4.body')
      },
      detection
    ];
  }

  async function startDetection(): Promise<void> {
    if (detectionStarted) return;
    detectionStarted = true;
    const result = await invokeValidated(
      'get_coverage',
      undefined,
      commandResponseSchemas.get_coverage
    );
    if (result.ok) {
      coverage = result.value;
      detectionStatus = result.value.tier === 'A' ? 'complete' : 'partial';
    } else {
      detectionStatus = 'partial';
    }
  }

  function persist(state: OnboardingState): void {
    onboardingState = state;
    void saveOnboardingState(state);
  }

  function onOnboardingStepChange(step: number): void {
    if (onboardingState === null) return;
    persist(advanceToSlide(onboardingState, step));
    if (step === 4) void startDetection();
  }

  function finishOnboarding(): void {
    if (onboardingState === null) return;
    persist(
      resolveOnboarding(onboardingState, 'completed', new Date().toISOString())
    );
  }

  function skipOnboarding(): void {
    if (onboardingState === null) return;
    persist(
      resolveOnboarding(onboardingState, 'skipped', new Date().toISOString())
    );
  }

  function dismissNotice(): void {
    if (onboardingState === null) return;
    persist(markNoticesSeen(onboardingState));
  }
</script>

{#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
{#snippet analysisIcon()}<NavIcon kind="analysis" />{/snippet}
{#snippet cpuIcon()}<NavIcon kind="cpu" />{/snippet}
{#snippet sessionsIcon()}<NavIcon kind="sessions" />{/snippet}
{#snippet guidedIcon()}<NavIcon kind="guided" />{/snippet}
{#snippet settingsIcon()}<NavIcon kind="settings" />{/snippet}

<div class="tw-ds app-shell tw-ambient">
  <TitleBar
    title={t('app.title')}
    {maximized}
    onMinimize={() => void windowAdapter.minimize()}
    onMaximizeToggle={() => {
      maximized = !maximized;
      void windowAdapter.toggleMaximize();
    }}
    onClose={() =>
      void windowAdapter.persistGeometry().then(() => windowAdapter.close())}
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
      {#if onboardingActive}
        <main class="onboarding" aria-label={t('onboarding.ariaLabel')}>
          <OnboardingFlow
            steps={onboardingSteps()}
            initialStep={initialStepFor(onboardingState!)}
            resumedNote={onboardingState!.last_slide > 1
              ? t('onboarding.resumed')
              : undefined}
            backLabel={t('onboarding.back')}
            nextLabel={t('onboarding.next')}
            finishLabel={t('onboarding.finish')}
            onSkip={skipOnboarding}
            skipLabel={t('onboarding.skip')}
            onFinish={finishOnboarding}
            onStepChange={onOnboardingStepChange}
            progressLabel={(step, count) =>
              t('onboarding.progress')
                .replace('{step}', String(step))
                .replace('{count}', String(count))}
          />
        </main>
      {:else if active === 'now'}
        {#if noticesVisible}
          <WhatsNewCards
            title={t('notices.title')}
            cards={[
              {
                id: 'low-level-access',
                title: t('notices.lowLevel.title'),
                body: t('notices.lowLevel.body')
              }
            ]}
            dismissLabel={t('notices.dismiss')}
            onDismiss={dismissNotice}
          />
        {/if}
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
  .onboarding {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: var(--space-6);
    box-sizing: border-box;
  }
  @media (max-width: 979px) {
    .sidebar {
      width: 48px;
      padding-inline: var(--space-2);
    }
  }
</style>
