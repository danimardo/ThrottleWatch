<script lang="ts">
  import { onMount } from 'svelte';
  import {
    BottomBar,
    Button,
    CloseBlockedDialog,
    EmptyState,
    FirstCloseDialog,
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
  import GuidedDiagnostic from '../features/guided/GuidedDiagnostic.svelte';
  import Analysis from '../features/analysis/Analysis.svelte';
  import CpuOverview from '../features/cpu/CpuOverview.svelte';
  import Sessions from '../features/sessions/Sessions.svelte';
  import SettingsHost from '../features/settings/SettingsHost.svelte';
  import {
    commandResponseSchemas,
    type CoverageMatrix,
    type OnboardingState
  } from '../lib/bridge/schemas';
  import { invokeValidated, listenValidated } from '../lib/bridge';
  import { stopOperationFor } from '../lib/lifecycle/close';
  import { createTranslator } from '../lib/i18n';
  import {
    isDetailedLoggingActive,
    setApplicationDetailedLogging
  } from '../lib/logging';
  import {
    createWindowAdapter,
    type WindowAdapter
  } from '../lib/bridge/window';
  import {
    advanceToSlide,
    initialStepFor,
    markNoticesSeen,
    repeatOnboarding,
    resolveOnboarding
  } from '../features/onboarding/model';
  import { pendingNoticeDefinitions } from '../features/onboarding/notices';
  import {
    loadOnboardingState,
    saveOnboardingState
  } from '../features/onboarding/repository';
  import {
    applyAppearance,
    listenToSystemAppearance,
    type ThemePreference
  } from '../features/appearance/appearance';
  import {
    applyGlassLevel,
    type GlassLevel,
    type MotionLevel
  } from '../design-system/tokens/tokens';
  import {
    stopGuidedSession,
    subscribeGuidedSession
  } from '../features/guided/session';

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

  let appearance = $state<{
    theme: ThemePreference;
    motion: MotionLevel;
    glass: GlassLevel | 'system';
  }>({ theme: 'system', motion: 'system', glass: 'system' });
  let active = $state<Destination['id']>('now');
  let maximized = $state(false);
  let windowAdapter: WindowAdapter = createWindowAdapter();
  let onboardingState = $state<OnboardingState | null>(null);
  let detectionStatus =
    $state<OnboardingDetectionContent['status']>('detecting');
  let coverage = $state<CoverageMatrix | null>(null);
  let guidedRunning = $state(false);
  let closeBlockedOpen = $state(false);
  let closeBlockedReason = $state<'guided' | 'export' | 'download' | 'install'>(
    'guided'
  );
  let firstCloseOpen = $state(false);
  let detectionStarted = false;
  const compactDestinations = destinations.slice(0, 3);
  const overflowDestinations = destinations.slice(3);

  let onboardingActive = $derived(onboardingState?.status === 'pending');
  let pendingNotices = $derived(
    onboardingState === null
      ? []
      : pendingNoticeDefinitions(onboardingState.last_seen_notice_version)
  );
  let noticesVisible = $derived(
    onboardingState !== null && !onboardingActive && pendingNotices.length > 0
  );

  onMount(() => {
    const onAppearanceChanged = (event: Event): void => {
      const detail = (event as CustomEvent<{ key: string; value: unknown }>)
        .detail;
      if (detail.key === 'appearance.theme') {
        appearance.theme = detail.value as ThemePreference;
      } else if (detail.key === 'appearance.motion') {
        appearance.motion = detail.value as MotionLevel;
      } else if (detail.key === 'appearance.glass') {
        appearance.glass = detail.value as GlassLevel | 'system';
      }
      applyStoredAppearance();
    };
    const onExportRequest = (): void => {
      active = 'sessions';
      window.setTimeout(() => {
        window.dispatchEvent(new CustomEvent('throttlewatch:open-export'));
      }, 0);
    };
    const onShortcut = (event: KeyboardEvent): void => {
      if (event.key === 'F1') {
        event.preventDefault();
        void invokeValidated(
          'open_external_url',
          { request: { target: 'help' } },
          commandResponseSchemas.open_external_url
        );
        return;
      }
      if (event.ctrlKey && !event.altKey && event.key.toLowerCase() === 'e') {
        event.preventDefault();
        active = 'sessions';
        window.setTimeout(() => {
          window.dispatchEvent(new CustomEvent('throttlewatch:open-export'));
        }, 0);
        return;
      }
      if (event.ctrlKey && !event.altKey && !event.shiftKey) {
        const destinationsByShortcut: Record<string, Destination['id']> = {
          '1': 'now',
          '2': 'analysis',
          '3': 'cpu',
          '4': 'sessions',
          '5': 'guided',
          '6': 'settings',
          ',': 'settings'
        };
        const destination = destinationsByShortcut[event.key];
        if (destination !== undefined) {
          event.preventDefault();
          active = destination;
        }
      }
    };
    window.addEventListener('keydown', onShortcut);
    window.addEventListener('throttlewatch:request-export', onExportRequest);
    windowAdapter = createWindowAdapter();
    void windowAdapter.isMaximized().then((value) => (maximized = value));
    void windowAdapter.restoreSavedGeometry();
    let stopGeometryListener: () => void = () => undefined;
    void windowAdapter
      .watchGeometryChanges(() => void windowAdapter.persistGeometry())
      .then((stop) => (stopGeometryListener = stop));
    applyStoredAppearance();
    const stopGuidedSessionListener = subscribeGuidedSession((phase) => {
      guidedRunning =
        phase !== null &&
        [
          'ready',
          'rest',
          'warming',
          'steady_load',
          'recovery',
          'cancelling'
        ].includes(phase.phase);
    });
    const stopAppearanceListener = listenToSystemAppearance('system', () => {
      if (appearance.theme === 'system') applyStoredAppearance();
    });
    window.addEventListener(
      'throttlewatch:appearance-changed',
      onAppearanceChanged
    );
    void loadOnboardingState().then((state) => {
      onboardingState = state;
      if (state.status === 'pending' && initialStepFor(state) === 4) {
        void startDetection();
      }
    });
    void loadDetailedLoggingState();
    void loadAppearanceState();
    let stopCloseBlockedListener: () => void = () => undefined;
    void listenValidated('lifecycle:close-blocked', (value) => {
      const reason = (value as { reason?: string }).reason;
      if (
        reason === 'guided' ||
        reason === 'export' ||
        reason === 'download' ||
        reason === 'install'
      ) {
        closeBlockedReason = reason;
        closeBlockedOpen = true;
      }
    }).then((stop) => (stopCloseBlockedListener = stop));
    let stopFirstCloseListener: () => void = () => undefined;
    void listenValidated('lifecycle:close-decision-required', () => {
      firstCloseOpen = true;
    }).then((stop) => (stopFirstCloseListener = stop));
    return () => {
      stopAppearanceListener();
      stopGeometryListener();
      stopGuidedSessionListener();
      stopCloseBlockedListener();
      stopFirstCloseListener();
      window.removeEventListener('keydown', onShortcut);
      window.removeEventListener(
        'throttlewatch:request-export',
        onExportRequest
      );
      window.removeEventListener(
        'throttlewatch:appearance-changed',
        onAppearanceChanged
      );
    };
  });

  function select(id: Destination['id']): void {
    active = id;
  }

  function closeWindow(): void {
    void windowAdapter.persistGeometry().then(() => windowAdapter.close());
  }

  function applyStoredAppearance(): void {
    applyAppearance(appearance.theme, appearance.motion, locale);
    applyGlassLevel(appearance.glass === 'system' ? 'full' : appearance.glass);
  }

  async function loadAppearanceState(): Promise<void> {
    const result = await invokeValidated(
      'get_preferences',
      undefined,
      commandResponseSchemas.get_preferences
    );
    if (!result.ok) return;
    const values = result.value.values;
    appearance = {
      theme: (values['appearance.theme'] as ThemePreference | undefined) ?? 'system',
      motion: (values['appearance.motion'] as MotionLevel | undefined) ?? 'system',
      glass:
        (values['appearance.glass'] as GlassLevel | 'system' | undefined) ??
        'system'
    };
    applyStoredAppearance();
  }

  async function loadDetailedLoggingState(): Promise<void> {
    const result = await invokeValidated(
      'get_preferences',
      undefined,
      commandResponseSchemas.get_preferences
    );
    if (result.ok) {
      setApplicationDetailedLogging(
        isDetailedLoggingActive(result.value.values['logging.detailed_until'])
      );
    }
  }

  async function resolveFirstClose(
    action: 'exit' | 'tray' | 'dismiss'
  ): Promise<void> {
    await invokeValidated(
      'resolve_first_close',
      { request: { action } },
      commandResponseSchemas.resolve_first_close
    );
    firstCloseOpen = false;
  }

  /** Whether it is safe to actually destroy the window right now — a running export/download/
   * install (T090), or the exit/tray/ask-once choice of FR-046 (T182) — is Rust's call, not this
   * button's: `windowAdapter.close()` emits the same native `CloseRequested` Alt+F4 or the window
   * manager would, so Rust can intercept it either way and answer with `lifecycle:close-blocked`
   * or `lifecycle:close-decision-required` instead of letting it close (see the listeners in
   * `onMount`) — that is the only gate for all four reasons plus the first-close choice.
   *
   * A running guided test asks first, locally: `guidedRunning` is already tracked here (for the
   * banner below) independently of anything the backend infers, so showing the dialog does not
   * need the round trip — Rust still refuses the native close on its own if this button is ever
   * bypassed (Alt+F4). */
  function requestClose(): void {
    if (guidedRunning) {
      closeBlockedReason = 'guided';
      closeBlockedOpen = true;
    } else {
      closeWindow();
    }
  }

  async function confirmClose(stopOperation: boolean): Promise<void> {
    await invokeValidated(
      'confirm_close',
      { request: { stop_operation: stopOperation } },
      commandResponseSchemas.confirm_close
    );
    closeBlockedOpen = false;
  }

  /** `install` never gets a confirm action here (see `closeBlockedConfirm`) or on the backend
   * (`lifecycle::confirm_close_outcome`), which refuses it independently of what this returns. */
  function closeBlockedConfirm(): (() => void) | undefined {
    if (closeBlockedReason === 'install') return undefined;
    return () => void confirmClose(stopOperationFor(closeBlockedReason));
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
          : coverage?.advanced_access === 'upgradable'
            ? t('onboarding.access.upgradable')
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

  function repeatIntroduction(): void {
    if (onboardingState === null) return;
    detectionStarted = false;
    detectionStatus = 'detecting';
    coverage = null;
    persist(repeatOnboarding(onboardingState));
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
    onClose={requestClose}
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
            cards={pendingNotices.map((notice) => ({
              id: notice.id,
              title: t(notice.titleKey),
              body: t(notice.bodyKey)
            }))}
            dismissLabel={t('notices.dismiss')}
            onDismiss={dismissNotice}
          />
        {/if}
        <Dashboard />
      {:else if active === 'guided'}
        <main aria-label={t('guided.ariaLabel')}>
          <GuidedDiagnostic />
        </main>
      {:else if active === 'analysis'}
        <main aria-label={t('nav.analysis')}>
          <Analysis />
        </main>
      {:else if active === 'cpu'}
        <main aria-label={t('cpu.title')}>
          <CpuOverview />
        </main>
      {:else if active === 'sessions'}
        <main aria-label={t('nav.sessions')}>
          <Sessions />
        </main>
      {:else if active === 'settings'}
        <main aria-label={t('nav.settings')}>
          <SettingsHost onRepeatIntroduction={repeatIntroduction} />
        </main>
      {:else}
        <EmptyState
          icon={active === 'sessions'
            ? sessionsIcon
            : active === 'settings'
              ? settingsIcon
              : nowIcon}
          title={t('screens.comingSoon')}
          description={t('screens.notAvailable')}
        />
      {/if}
    </div>
  </div>

  {#if guidedRunning && active !== 'guided'}
    <div class="guided-session-banner" role="status">
      <span>{t('guided.inProgress')}</span>
      <Button
        variant="secondary"
        label={t('guided.stop')}
        onclick={() => void stopGuidedSession()}
      />
    </div>
  {/if}

  <CloseBlockedDialog
    bind:open={closeBlockedOpen}
    reason={closeBlockedReason}
    title={closeBlockedReason === 'guided'
      ? t('guided.inProgress')
      : closeBlockedReason === 'export'
        ? t('settings.closeBlockedExportTitle')
        : closeBlockedReason === 'download'
          ? t('settings.closeBlockedDownloadTitle')
          : t('settings.closeBlockedInstallTitle')}
    description={closeBlockedReason === 'guided'
      ? t('guided.closeDescription')
      : closeBlockedReason === 'export'
        ? t('settings.closeBlockedExportDescription')
        : closeBlockedReason === 'download'
          ? t('settings.closeBlockedDownloadDescription')
          : t('settings.closeBlockedInstallDescription')}
    confirmLabel={closeBlockedReason === 'guided'
      ? t('guided.closeAndStop')
      : closeBlockedReason === 'export'
        ? t('settings.closeBlockedExportConfirm')
        : closeBlockedReason === 'download'
          ? t('settings.closeBlockedDownloadConfirm')
          : undefined}
    cancelLabel={closeBlockedReason === 'guided'
      ? t('guided.cancel')
      : closeBlockedReason === 'install'
        ? t('settings.closeBlockedInstallUnderstood')
        : t('settings.cancel')}
    onConfirm={closeBlockedConfirm()}
    onCancel={() => (closeBlockedOpen = false)}
  />

  <FirstCloseDialog
    bind:open={firstCloseOpen}
    title={t('settings.firstCloseTitle')}
    description={t('settings.firstCloseDescription')}
    exitLabel={t('settings.exit')}
    trayLabel={t('settings.firstCloseTray')}
    trayNote={t('settings.firstCloseTrayNote')}
    settingsHint={t('settings.firstCloseSettingsHint')}
    onExit={() => void resolveFirstClose('exit')}
    onTray={() => void resolveFirstClose('tray')}
    onDismiss={() => void resolveFirstClose('dismiss')}
  />

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
    /* `background-color`, not the `background` shorthand: the shorthand resets
       `background-image` (and `-attachment`/`-size`) to their initial values,
       which silently erased the `.tw-ambient` gradient this element also
       carries — same specificity fight `design/mockup/src/App.svelte` avoids
       by never re-declaring `background` on its own shell root. */
    background-color: var(--bg);
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
  .guided-session-banner {
    position: fixed;
    left: var(--space-4);
    right: var(--space-4);
    bottom: var(--space-4);
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    color: var(--text-primary);
    background: var(--surface-raised);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
  }
  .sidebar {
    width: 196px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4) var(--space-3);
    border-right: 1px solid var(--glass-border);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
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
