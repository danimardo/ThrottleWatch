<script lang="ts">
  /**
   * The full "Ajustes" screen: General, Idioma, Apariencia,
   * Monitorización, Bandeja y notificaciones, Datos y privacidad,
   * Sensores y cobertura, Diagnóstico, Actualizaciones, Acerca de y
   * ayuda, Zona de riesgo — built entirely from existing primitives
   * (OptionRow, Switch, SegmentedControl, Select, Button, Dialog,
   * ProgressBar, Banner). None of those primitives are modified or
   * reimplemented here.
   *
   * Strictly presentational: every current value, every option list,
   * every disabled/reason pair, and every status (sensor coverage,
   * update progress, delete/reset progress) is a prop. This component
   * never decides *why* a control is disabled or *what* an update
   * check finds — the host app computes that and passes it in. The
   * only local state is UI-only interaction state: whether the two
   * destructive-action confirmation dialogs are open, and which
   * per-section "Avanzado" blocks are unfolded (never persisted —
   * `data-model.md` deliberately has no key for it).
   *
   * Shape decisions that come straight from the spec (2026-09-18
   * review, gap-analysis.md §10), so don't "simplify" them away:
   * - the close action is tri-state (`unset | exit | tray`), not a
   *   boolean: while `unset`, no segment is selected and
   *   `closeActionUndecidedText` explains the question will be asked
   *   on first close (FR-046);
   * - there is NO crash/usage reporting switch and NO update channel —
   *   the product has zero network besides the opt-in updater;
   * - updates have separate Download and Install gestures with a
   *   `verified` state in between (FR-054);
   * - the low-level access is the 5-value enum from lib/access.ts;
   * - advanced rows (exact interval, per-core history, quiet period,
   *   log level) live in a folded block per section (FR-049).
   *
   * No copy is hardcoded: every visible string is a prop, so this
   * component carries no embedded ES/EN text (rule 2).
   *
   * `SettingsScreen` does not scroll itself; wrap it in a scrolling
   * container in the host shell if the composed sections exceed the
   * viewport (screens generally will).
   */
  import type { Snippet } from 'svelte';
  import type { AdvancedAccessState } from '../lib/access';
  import OptionRow from './OptionRow.svelte';
  import Switch from './Switch.svelte';
  import SegmentedControl from './SegmentedControl.svelte';
  import Select from './Select.svelte';
  import Button from './Button.svelte';
  import Dialog from './Dialog.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Banner from './Banner.svelte';

  export interface SettingsOption {
    value: string;
    label: string;
  }

  export type SettingsUpdateStatus =
    | 'idle'
    | 'checking'
    | 'upToDate'
    | 'available'
    | 'downloading'
    | 'verified'
    | 'installing'
    | 'error';
  export type SettingsSensorStatus = 'detecting' | 'complete' | 'partial';
  export type SettingsDangerStatus = 'idle' | 'inProgress' | 'error';
  export type SettingsCloseAction = 'unset' | 'exit' | 'tray';

  export interface SettingsGeneralSection {
    title: string;
    closeActionRowLabel: string;
    closeActionRowDescription?: string;
    /** aria-label of the SegmentedControl */
    closeActionLabel: string;
    closeAction: SettingsCloseAction;
    /** Exactly two options, values `exit` and `tray`. */
    closeActionOptions: SettingsOption[];
    /** Shown instead of the description while `closeAction === 'unset'`. */
    closeActionUndecidedText: string;
    onCloseActionChange: (value: 'exit' | 'tray') => void;
    startOnLogin: boolean;
    startOnLoginLabel: string;
    startOnLoginDescription?: string;
    onStartOnLoginChange: (checked: boolean) => void;
    startHiddenLabel: string;
    startHiddenDescription?: string;
    startHiddenInTray: boolean;
    onStartHiddenInTrayChange: (checked: boolean) => void;
    startHiddenDisabled?: boolean;
    startHiddenDisabledReason?: string;
  }

  export interface SettingsLanguageSection {
    title: string;
    rowLabel: string;
    rowDescription?: string;
    selectLabel: string;
    value: string;
    options: SettingsOption[];
    onChange: (value: string) => void;
    /** e.g. "Idioma efectivo: Español (desde Windows: ca-ES)". */
    effectiveLabel?: string;
  }

  export interface SettingsAppearanceSection {
    title: string;
    rowLabel: string;
    rowDescription?: string;
    segmentedLabel: string;
    theme: string;
    options: SettingsOption[];
    onThemeChange: (value: string) => void;
    motionRowLabel: string;
    motionRowDescription?: string;
    motionLabel: string;
    motion: string;
    motionOptions: SettingsOption[];
    onMotionChange: (value: string) => void;
    /** Glass material level: system (follows Windows "Transparency effects") / full / reduced / off. */
    glassRowLabel: string;
    glassRowDescription?: string;
    glassLabel: string;
    glass: string;
    glassOptions: SettingsOption[];
    onGlassChange: (value: string) => void;
  }

  export interface SettingsMonitoringSection {
    title: string;
    modeRowLabel: string;
    modeRowDescription?: string;
    modeLabel: string;
    mode: string;
    modeOptions: SettingsOption[];
    onModeChange: (value: string) => void;
    onBatteryRowLabel: string;
    onBatteryRowDescription?: string;
    onBatteryLabel: string;
    onBattery: string;
    onBatteryOptions: SettingsOption[];
    onOnBatteryChange: (value: string) => void;
    /** Label of the folded "Avanzado" disclosure. */
    advancedLabel: string;
    intervalRowLabel: string;
    intervalRowDescription?: string;
    intervalSelectLabel: string;
    samplingInterval: string;
    samplingIntervalOptions: SettingsOption[];
    onSamplingIntervalChange: (value: string) => void;
    intervalDisabled?: boolean;
    intervalDisabledReason?: string;
    perCoreHistoryLabel: string;
    perCoreHistoryDescription?: string;
    perCoreHistory: boolean;
    onPerCoreHistoryChange: (checked: boolean) => void;
  }

  export interface SettingsTraySection {
    title: string;
    backgroundLabel: string;
    backgroundDescription?: string;
    backgroundMonitoring: boolean;
    onBackgroundMonitoringChange: (checked: boolean) => void;
    notificationsLabel: string;
    notificationsDescription?: string;
    notifications: boolean;
    onNotificationsChange: (checked: boolean) => void;
    testNotificationLabel?: string;
    onTestNotification?: () => void;
    testNotificationDisabled?: boolean;
    testNotificationDisabledReason?: string;
    advancedLabel: string;
    quietPeriodLabel: string;
    quietPeriodDescription?: string;
    quietPeriodEnabled: boolean;
    onQuietPeriodEnabledChange: (checked: boolean) => void;
    quietPeriodStartLabel: string;
    quietPeriodEndLabel: string;
    quietPeriodStart: string;
    quietPeriodEnd: string;
    /** Hour options, e.g. { value: '22', label: '22:00' }. */
    quietPeriodOptions: SettingsOption[];
    onQuietPeriodChange: (start: string, end: string) => void;
    /** Read-only explanation of persistence/cooldown rules (values come from the ruleset). */
    alertRulesNote?: string;
  }

  export interface SettingsPrivacySection {
    title: string;
    anonymizeExportsLabel: string;
    anonymizeExportsDescription?: string;
    anonymizeExports: boolean;
    onAnonymizeExportsChange: (checked: boolean) => void;
    retentionRowLabel: string;
    retentionRowDescription?: string;
    retentionSelectLabel: string;
    dataRetention: string;
    /** Exactly the spec's four: session / 1d / 7d / 30d. */
    dataRetentionOptions: SettingsOption[];
    onDataRetentionChange: (value: string) => void;
    storageRowLabel: string;
    /** e.g. "38 MB" */
    storageValue: string;
    /** e.g. "Base de datos 31 MB · Registros 7 MB · 12 sesiones · desde el 10 sept" */
    storageDescription?: string;
    /**
     * FR-075: set only on the run that just recovered from a corrupt database — offers
     * exporting the file moved aside instead of silently forgetting it happened.
     */
    corruptBackupRowLabel: string;
    corruptBackupNotice?: string;
    corruptBackupExportLabel: string;
    onExportCorruptBackup: () => void;
    exportRowLabel: string;
    exportRowDescription?: string;
    exportButtonLabel: string;
    onExport: () => void;
    exportDisabled?: boolean;
    exportDisabledReason?: string;
    deleteRowLabel: string;
    deleteRowDescription?: string;
    deleteButtonLabel: string;
    deleteStatus?: SettingsDangerStatus;
    deleteErrorMessage?: string;
    onDeleteAllData: () => void;
    deleteDialogTitle: string;
    deleteDialogDescription: string;
    deleteDialogCancelLabel: string;
    deleteDialogConfirmLabel: string;
  }

  export interface SettingsSensorsSection {
    title: string;
    rowLabel: string;
    status: SettingsSensorStatus;
    detectingLabel: string;
    coverageTitle?: string;
    coverageDescription?: string;
    advancedAccess?: AdvancedAccessState;
    advancedAccessNote?: string;
    requestAccessLabel?: string;
    onRequestAdvancedAccess?: () => void;
    accessRetryLabel?: string;
    onAccessRetry?: () => void;
    /** `denied` (data-model.md): a policy or antivirus blocks it — a link to help, not a retry. */
    accessHelpLabel?: string;
    onAccessHelp?: () => void;
    disableAccessLabel?: string;
    onDisableAdvancedAccess?: () => void;
    recheckLabel: string;
    onRecheck: () => void;
    /** Host renders a `CoverageMatrix` here; when present it replaces the short coverage line. */
    coverage?: Snippet;
  }

  export interface SettingsSafetyLimit {
    label: string;
    value: string;
  }

  export interface SettingsDiagnosticsSection {
    title: string;
    durationRowLabel: string;
    durationRowDescription?: string;
    durationLabel: string;
    duration: string;
    /** short / standard / long */
    durationOptions: SettingsOption[];
    onDurationChange: (value: string) => void;
    requireAcLabel: string;
    requireAcDescription?: string;
    requireAc: boolean;
    onRequireAcChange: (checked: boolean) => void;
    notifyLabel: string;
    notifyDescription?: string;
    notifyOnFinish: boolean;
    onNotifyOnFinishChange: (checked: boolean) => void;
    notifyDisabled?: boolean;
    notifyDisabledReason?: string;
    safetyLimitsTitle: string;
    /** Read-only: stop temperature, minimum headroom, max duration, watchdog. */
    safetyLimits: SettingsSafetyLimit[];
    safetyLimitsNote?: string;
  }

  export interface SettingsUpdatesSection {
    title: string;
    versionRowLabel: string;
    currentVersion: string;
    autoCheckLabel: string;
    autoCheckDescription?: string;
    autoCheck: boolean;
    onAutoCheckChange: (checked: boolean) => void;
    lastCheckLabel?: string;
    lastCheckValue?: string;
    status: SettingsUpdateStatus;
    checkNowLabel: string;
    onCheckNow: () => void;
    checkNowDisabled?: boolean;
    checkNowDisabledReason?: string;
    checkingLabel?: string;
    upToDateLabel?: string;
    availableVersion?: string;
    availableDescription?: string;
    downloadLabel?: string;
    onDownload?: () => void;
    downloadPercent?: number;
    downloadingLabel?: string;
    verifiedLabel?: string;
    verifiedDescription?: string;
    installLabel?: string;
    onInstall?: () => void;
    installDisabled?: boolean;
    installDisabledReason?: string;
    installingLabel?: string;
    errorMessage?: string;
    retryLabel?: string;
    onRetry?: () => void;
  }

  export interface SettingsAboutLink {
    label: string;
    onOpen: () => void;
    /** Opens the system browser: rendered with an external-link glyph and `externalHint`. */
    external?: boolean;
    externalHint?: string;
  }

  export interface SettingsAboutSection {
    title: string;
    versionRowLabel: string;
    version: string;
    buildLabel?: string;
    repeatIntroLabel: string;
    repeatIntroDescription?: string;
    onRepeatIntro: () => void;
    links: SettingsAboutLink[];
    /** Optional folded block with the time-limited detailed logging switch. */
    advancedLabel?: string;
    detailedLoggingRowLabel?: string;
    detailedLoggingRowDescription?: string;
    detailedLoggingLabel?: string;
    detailedLogging: boolean;
    detailedLoggingUntilLabel?: string;
    onDetailedLoggingChange: (checked: boolean) => void;
    /** Host renders a `TechnicalSummary` here. */
    technicalSummary?: Snippet;
  }

  export interface SettingsRiskZoneSection {
    title: string;
    resetRowLabel: string;
    resetRowDescription?: string;
    resetButtonLabel: string;
    resetStatus?: SettingsDangerStatus;
    resetErrorMessage?: string;
    onReset: () => void;
    resetDialogTitle: string;
    resetDialogDescription: string;
    resetDialogCancelLabel: string;
    resetDialogConfirmLabel: string;
  }

  interface Props {
    general: SettingsGeneralSection;
    language: SettingsLanguageSection;
    appearance: SettingsAppearanceSection;
    monitoring: SettingsMonitoringSection;
    tray: SettingsTraySection;
    privacy: SettingsPrivacySection;
    sensors: SettingsSensorsSection;
    diagnostics: SettingsDiagnosticsSection;
    updates: SettingsUpdatesSection;
    about: SettingsAboutSection;
    riskZone: SettingsRiskZoneSection;
    /** Extra content appended after "Zona de riesgo" (e.g. a build id). */
    footer?: Snippet;
  }

  let {
    general,
    language,
    appearance,
    monitoring,
    tray,
    privacy,
    sensors,
    diagnostics,
    updates,
    about,
    riskZone,
    footer
  }: Props = $props();

  let deleteDialogOpen = $state(false);
  let resetDialogOpen = $state(false);

  /** Per-section fold state of the "Avanzado" blocks — view-only, never persisted. */
  let advancedOpen: Record<'monitoring' | 'tray' | 'about', boolean> = $state({
    monitoring: false,
    tray: false,
    about: false
  });

  function confirmDelete() {
    deleteDialogOpen = false;
    privacy.onDeleteAllData();
  }
  function confirmReset() {
    resetDialogOpen = false;
    riskZone.onReset();
  }

  let updatesBusy = $derived(
    updates.status === 'checking' ||
      updates.status === 'downloading' ||
      updates.status === 'installing'
  );
</script>

{#snippet sensorsCheckmark()}
  <!--
    Local shape, deliberately not StatusIcon — same rationale as
    OnboardingFlow's own slide-5 checkmark and Banner's checkmark:
    StatusIcon's glyphs are reserved for diagnostic_report.classification.
  -->
  <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
    <circle
      cx="12"
      cy="12"
      r="10"
      fill="none"
      stroke="var(--status-normal)"
      stroke-width="1.6"
    />
    <path
      d="M7.5 12.5 L10.5 15.5 L16.5 8.5"
      fill="none"
      stroke="var(--status-normal)"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
{/snippet}

{#snippet sensorsRetryAction()}
  <Button
    variant="secondary"
    label={sensors.recheckLabel}
    onclick={sensors.onRecheck}
  />
{/snippet}

{#snippet sensorsRequestAccessAction()}
  <Button
    variant="secondary"
    label={sensors.requestAccessLabel ?? ''}
    onclick={sensors.onRequestAdvancedAccess}
  />
{/snippet}

{#snippet sensorsAccessRetryAction()}
  <Button
    variant="secondary"
    label={sensors.accessRetryLabel ?? ''}
    onclick={sensors.onAccessRetry}
  />
{/snippet}

{#snippet sensorsAccessHelpAction()}
  <Button
    variant="secondary"
    label={sensors.accessHelpLabel ?? ''}
    onclick={sensors.onAccessHelp}
  />
{/snippet}

{#snippet sensorsDisableAccessAction()}
  <Button
    variant="secondary"
    label={sensors.disableAccessLabel ?? ''}
    onclick={sensors.onDisableAdvancedAccess}
  />
{/snippet}

{#snippet advancedToggle(key: 'monitoring' | 'tray' | 'about', label: string)}
  <button
    type="button"
    class="disclosure caption"
    aria-expanded={advancedOpen[key]}
    onclick={() => (advancedOpen[key] = !advancedOpen[key])}
  >
    <span class="chevron" class:open={advancedOpen[key]} aria-hidden="true">
      <svg viewBox="0 0 12 12" width="10" height="10"
        ><path
          d="M4 2 L8 6 L4 10"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        /></svg
      >
    </span>
    {label}
  </button>
{/snippet}

<div class="tw-ds tw-settings-screen">
  <!-- 1. General -->
  <section>
    <h3 class="label section-title">{general.title}</h3>
    <OptionRow
      label={general.closeActionRowLabel}
      description={general.closeAction === 'unset'
        ? general.closeActionUndecidedText
        : general.closeActionRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={general.closeActionLabel}
          value={general.closeAction === 'unset' ? '' : general.closeAction}
          options={general.closeActionOptions}
          onchange={(v) => general.onCloseActionChange(v as 'exit' | 'tray')}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={general.startOnLoginLabel}
      description={general.startOnLoginDescription}
    >
      {#snippet control()}
        <Switch
          checked={general.startOnLogin}
          label={general.startOnLoginLabel}
          onchange={general.onStartOnLoginChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={general.startHiddenLabel}
      description={general.startHiddenDescription}
      disabled={general.startHiddenDisabled}
      disabledReason={general.startHiddenDisabledReason}
    >
      {#snippet control()}
        <Switch
          checked={general.startHiddenInTray}
          label={general.startHiddenLabel}
          disabled={general.startHiddenDisabled}
          onchange={general.onStartHiddenInTrayChange}
        />
      {/snippet}
    </OptionRow>
  </section>

  <!-- 2. Idioma -->
  <section>
    <h3 class="label section-title">{language.title}</h3>
    <OptionRow label={language.rowLabel} description={language.rowDescription}>
      {#snippet control()}
        <Select
          label={language.selectLabel}
          value={language.value}
          options={language.options}
          onchange={language.onChange}
        />
      {/snippet}
    </OptionRow>
    {#if language.effectiveLabel}
      <span class="caption effective" style:color="var(--text-tertiary)"
        >{language.effectiveLabel}</span
      >
    {/if}
  </section>

  <!-- 3. Apariencia -->
  <section>
    <h3 class="label section-title">{appearance.title}</h3>
    <OptionRow
      label={appearance.rowLabel}
      description={appearance.rowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={appearance.segmentedLabel}
          value={appearance.theme}
          options={appearance.options}
          onchange={appearance.onThemeChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={appearance.motionRowLabel}
      description={appearance.motionRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={appearance.motionLabel}
          value={appearance.motion}
          options={appearance.motionOptions}
          onchange={appearance.onMotionChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={appearance.glassRowLabel}
      description={appearance.glassRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={appearance.glassLabel}
          value={appearance.glass}
          options={appearance.glassOptions}
          onchange={appearance.onGlassChange}
        />
      {/snippet}
    </OptionRow>
  </section>

  <!-- 4. Monitorización -->
  <section>
    <h3 class="label section-title">{monitoring.title}</h3>
    <OptionRow
      label={monitoring.modeRowLabel}
      description={monitoring.modeRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={monitoring.modeLabel}
          value={monitoring.mode}
          options={monitoring.modeOptions}
          onchange={monitoring.onModeChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={monitoring.onBatteryRowLabel}
      description={monitoring.onBatteryRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={monitoring.onBatteryLabel}
          value={monitoring.onBattery}
          options={monitoring.onBatteryOptions}
          onchange={monitoring.onOnBatteryChange}
        />
      {/snippet}
    </OptionRow>
    {@render advancedToggle('monitoring', monitoring.advancedLabel)}
    {#if advancedOpen.monitoring}
      <div class="advanced">
        <OptionRow
          label={monitoring.intervalRowLabel}
          description={monitoring.intervalRowDescription}
          disabled={monitoring.intervalDisabled}
          disabledReason={monitoring.intervalDisabledReason}
        >
          {#snippet control()}
            <Select
              label={monitoring.intervalSelectLabel}
              value={monitoring.samplingInterval}
              options={monitoring.samplingIntervalOptions}
              disabled={monitoring.intervalDisabled}
              onchange={monitoring.onSamplingIntervalChange}
            />
          {/snippet}
        </OptionRow>
        <OptionRow
          label={monitoring.perCoreHistoryLabel}
          description={monitoring.perCoreHistoryDescription}
        >
          {#snippet control()}
            <Switch
              checked={monitoring.perCoreHistory}
              label={monitoring.perCoreHistoryLabel}
              onchange={monitoring.onPerCoreHistoryChange}
            />
          {/snippet}
        </OptionRow>
      </div>
    {/if}
  </section>

  <!-- 5. Bandeja y notificaciones -->
  <section>
    <h3 class="label section-title">{tray.title}</h3>
    <OptionRow
      label={tray.backgroundLabel}
      description={tray.backgroundDescription}
    >
      {#snippet control()}
        <Switch
          checked={tray.backgroundMonitoring}
          label={tray.backgroundLabel}
          onchange={tray.onBackgroundMonitoringChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={tray.notificationsLabel}
      description={tray.notificationsDescription}
    >
      {#snippet control()}
        <Switch
          checked={tray.notifications}
          label={tray.notificationsLabel}
          onchange={tray.onNotificationsChange}
        />
      {/snippet}
    </OptionRow>
    {#if tray.onTestNotification && tray.testNotificationLabel}
      <OptionRow
        label={tray.testNotificationLabel}
        disabled={tray.testNotificationDisabled}
        disabledReason={tray.testNotificationDisabledReason}
      >
        {#snippet control()}
          <Button
            variant="secondary"
            label={tray.testNotificationLabel ?? ''}
            disabled={tray.testNotificationDisabled}
            onclick={tray.onTestNotification}
          />
        {/snippet}
      </OptionRow>
    {/if}
    {@render advancedToggle('tray', tray.advancedLabel)}
    {#if advancedOpen.tray}
      <div class="advanced">
        <OptionRow
          label={tray.quietPeriodLabel}
          description={tray.quietPeriodDescription}
        >
          {#snippet control()}
            <Switch
              checked={tray.quietPeriodEnabled}
              label={tray.quietPeriodLabel}
              onchange={tray.onQuietPeriodEnabledChange}
            />
          {/snippet}
        </OptionRow>
        <OptionRow
          label={tray.quietPeriodStartLabel}
          disabled={!tray.quietPeriodEnabled}
        >
          {#snippet control()}
            <Select
              label={tray.quietPeriodStartLabel}
              value={tray.quietPeriodStart}
              options={tray.quietPeriodOptions}
              disabled={!tray.quietPeriodEnabled}
              onchange={(v) => tray.onQuietPeriodChange(v, tray.quietPeriodEnd)}
            />
          {/snippet}
        </OptionRow>
        <OptionRow
          label={tray.quietPeriodEndLabel}
          disabled={!tray.quietPeriodEnabled}
        >
          {#snippet control()}
            <Select
              label={tray.quietPeriodEndLabel}
              value={tray.quietPeriodEnd}
              options={tray.quietPeriodOptions}
              disabled={!tray.quietPeriodEnabled}
              onchange={(v) =>
                tray.onQuietPeriodChange(tray.quietPeriodStart, v)}
            />
          {/snippet}
        </OptionRow>
        {#if tray.alertRulesNote}
          <span class="caption note" style:color="var(--text-tertiary)"
            >{tray.alertRulesNote}</span
          >
        {/if}
      </div>
    {/if}
  </section>

  <!-- 6. Datos y privacidad -->
  <section>
    <h3 class="label section-title">{privacy.title}</h3>
    <OptionRow
      label={privacy.retentionRowLabel}
      description={privacy.retentionRowDescription}
    >
      {#snippet control()}
        <Select
          label={privacy.retentionSelectLabel}
          value={privacy.dataRetention}
          options={privacy.dataRetentionOptions}
          onchange={privacy.onDataRetentionChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={privacy.storageRowLabel}
      description={privacy.storageDescription}
    >
      {#snippet control()}
        <span class="body value" style:color="var(--text-secondary)"
          >{privacy.storageValue}</span
        >
      {/snippet}
    </OptionRow>
    {#if privacy.corruptBackupNotice}
      <OptionRow
        label={privacy.corruptBackupRowLabel}
        description={privacy.corruptBackupNotice}
      >
        {#snippet control()}
          <Button
            variant="secondary"
            label={privacy.corruptBackupExportLabel}
            onclick={privacy.onExportCorruptBackup}
          />
        {/snippet}
      </OptionRow>
    {/if}
    <OptionRow
      label={privacy.anonymizeExportsLabel}
      description={privacy.anonymizeExportsDescription}
    >
      {#snippet control()}
        <Switch
          checked={privacy.anonymizeExports}
          label={privacy.anonymizeExportsLabel}
          onchange={privacy.onAnonymizeExportsChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={privacy.exportRowLabel}
      description={privacy.exportRowDescription}
      disabled={privacy.exportDisabled}
      disabledReason={privacy.exportDisabledReason}
    >
      {#snippet control()}
        <Button
          variant="secondary"
          label={privacy.exportButtonLabel}
          disabled={privacy.exportDisabled}
          onclick={privacy.onExport}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={privacy.deleteRowLabel}
      description={privacy.deleteRowDescription}
    >
      {#snippet control()}
        <Button
          variant="secondary"
          label={privacy.deleteButtonLabel}
          disabled={privacy.deleteStatus === 'inProgress'}
          onclick={() => (deleteDialogOpen = true)}
        />
      {/snippet}
    </OptionRow>
    {#if privacy.deleteStatus === 'inProgress'}
      <div class="status-block">
        <ProgressBar
          indeterminate
          tone="accent"
          label={privacy.deleteButtonLabel}
        />
      </div>
    {:else if privacy.deleteStatus === 'error'}
      <div class="status-block">
        <Banner tone="critical" title={privacy.deleteErrorMessage ?? ''} />
      </div>
    {/if}
  </section>

  <!-- 7. Sensores y cobertura -->
  <section>
    <h3 class="label section-title">{sensors.title}</h3>
    <OptionRow label={sensors.rowLabel}>
      {#snippet control()}
        <Button
          variant="secondary"
          label={sensors.recheckLabel}
          onclick={sensors.onRecheck}
          disabled={sensors.status === 'detecting'}
        />
      {/snippet}
    </OptionRow>
    <div class="status-block">
      {#if sensors.status === 'detecting'}
        <ProgressBar
          indeterminate
          tone="accent"
          label={sensors.detectingLabel}
        />
        <span class="caption" style:color="var(--text-tertiary)"
          >{sensors.detectingLabel}</span
        >
      {:else if sensors.coverage}
        {@render sensors.coverage()}
      {:else if sensors.status === 'complete'}
        <div class="sensors-complete">
          {@render sensorsCheckmark()}
          <div class="sensors-complete-text">
            <span class="body-strong" style:color="var(--text-primary)"
              >{sensors.coverageTitle}</span
            >
            {#if sensors.coverageDescription}
              <span class="caption" style:color="var(--text-secondary)"
                >{sensors.coverageDescription}</span
              >
            {/if}
          </div>
        </div>
      {:else}
        <Banner
          tone="warning"
          title={sensors.coverageTitle ?? ''}
          description={sensors.coverageDescription}
          action={sensorsRetryAction}
        />
      {/if}
      {#if sensors.status !== 'detecting' && !sensors.coverage}
        {#if sensors.advancedAccess === 'installable' || sensors.advancedAccess === 'upgradable'}
          <Banner
            tone="info"
            title={sensors.advancedAccessNote ?? ''}
            action={sensors.onRequestAdvancedAccess
              ? sensorsRequestAccessAction
              : undefined}
          />
        {:else if sensors.advancedAccess === 'denied'}
          <Banner
            tone="warning"
            title={sensors.advancedAccessNote ?? ''}
            action={sensors.onAccessHelp ? sensorsAccessHelpAction : undefined}
          />
        {:else if sensors.advancedAccess === 'error'}
          <Banner
            tone="critical"
            title={sensors.advancedAccessNote ?? ''}
            action={sensors.onAccessRetry
              ? sensorsAccessRetryAction
              : undefined}
          />
        {:else if sensors.advancedAccess === 'available' && sensors.onDisableAdvancedAccess}
          {@render sensorsDisableAccessAction()}
        {:else if sensors.advancedAccessNote}
          <span class="caption" style:color="var(--text-tertiary)"
            >{sensors.advancedAccessNote}</span
          >
        {/if}
      {/if}
    </div>
  </section>

  <!-- 8. Diagnóstico -->
  <section>
    <h3 class="label section-title">{diagnostics.title}</h3>
    <OptionRow
      label={diagnostics.durationRowLabel}
      description={diagnostics.durationRowDescription}
    >
      {#snippet control()}
        <SegmentedControl
          label={diagnostics.durationLabel}
          value={diagnostics.duration}
          options={diagnostics.durationOptions}
          onchange={diagnostics.onDurationChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={diagnostics.requireAcLabel}
      description={diagnostics.requireAcDescription}
    >
      {#snippet control()}
        <Switch
          checked={diagnostics.requireAc}
          label={diagnostics.requireAcLabel}
          onchange={diagnostics.onRequireAcChange}
        />
      {/snippet}
    </OptionRow>
    <OptionRow
      label={diagnostics.notifyLabel}
      description={diagnostics.notifyDescription}
      disabled={diagnostics.notifyDisabled}
      disabledReason={diagnostics.notifyDisabledReason}
    >
      {#snippet control()}
        <Switch
          checked={diagnostics.notifyOnFinish}
          label={diagnostics.notifyLabel}
          disabled={diagnostics.notifyDisabled}
          onchange={diagnostics.onNotifyOnFinishChange}
        />
      {/snippet}
    </OptionRow>
    <div class="safety">
      <span class="caption" style:color="var(--text-tertiary)"
        >{diagnostics.safetyLimitsTitle}</span
      >
      <dl class="safety-list">
        {#each diagnostics.safetyLimits as limit (limit.label)}
          <div class="safety-row">
            <dt class="body" style:color="var(--text-secondary)">
              {limit.label}
            </dt>
            <dd class="body value" style:color="var(--text-primary)">
              {limit.value}
            </dd>
          </div>
        {/each}
      </dl>
      {#if diagnostics.safetyLimitsNote}
        <span class="caption" style:color="var(--text-tertiary)"
          >{diagnostics.safetyLimitsNote}</span
        >
      {/if}
    </div>
  </section>

  <!-- 9. Actualizaciones -->
  <section>
    <h3 class="label section-title">{updates.title}</h3>
    <OptionRow label={updates.versionRowLabel}>
      {#snippet control()}
        <span class="body value" style:color="var(--text-secondary)"
          >{updates.currentVersion}</span
        >
      {/snippet}
    </OptionRow>
    <OptionRow
      label={updates.autoCheckLabel}
      description={updates.autoCheckDescription}
    >
      {#snippet control()}
        <Switch
          checked={updates.autoCheck}
          label={updates.autoCheckLabel}
          onchange={updates.onAutoCheckChange}
        />
      {/snippet}
    </OptionRow>
    {#if updates.lastCheckLabel}
      <OptionRow label={updates.lastCheckLabel}>
        {#snippet control()}
          <span class="body value" style:color="var(--text-secondary)"
            >{updates.lastCheckValue ?? '—'}</span
          >
        {/snippet}
      </OptionRow>
    {/if}
    <OptionRow
      label={updates.checkNowLabel}
      disabled={updates.checkNowDisabled}
      disabledReason={updates.checkNowDisabledReason}
    >
      {#snippet control()}
        <Button
          variant="secondary"
          label={updates.checkNowLabel}
          disabled={updatesBusy || updates.checkNowDisabled}
          onclick={updates.onCheckNow}
        />
      {/snippet}
    </OptionRow>
    <div class="status-block">
      {#if updates.status === 'checking'}
        <ProgressBar
          indeterminate
          tone="accent"
          label={updates.checkingLabel}
        />
        {#if updates.checkingLabel}
          <span class="caption" style:color="var(--text-tertiary)"
            >{updates.checkingLabel}</span
          >
        {/if}
      {:else if updates.status === 'upToDate' && updates.upToDateLabel}
        <Banner tone="info" title={updates.upToDateLabel} />
      {:else if updates.status === 'available'}
        {#snippet downloadAction()}
          <Button
            variant="primary"
            label={updates.downloadLabel ?? ''}
            onclick={updates.onDownload}
          />
        {/snippet}
        <Banner
          tone="info"
          title={updates.availableVersion ?? ''}
          description={updates.availableDescription}
          action={updates.onDownload ? downloadAction : undefined}
        />
      {:else if updates.status === 'downloading'}
        <ProgressBar
          percent={updates.downloadPercent ?? 0}
          tone="accent"
          label={updates.downloadingLabel}
        />
        {#if updates.downloadingLabel}
          <span class="caption" style:color="var(--text-tertiary)"
            >{updates.downloadingLabel} — {updates.downloadPercent ?? 0}%</span
          >
        {/if}
      {:else if updates.status === 'verified'}
        {#snippet installAction()}
          <div class="install-action">
            <Button
              variant="primary"
              label={updates.installLabel ?? ''}
              disabled={updates.installDisabled}
              onclick={updates.onInstall}
            />
            {#if updates.installDisabled && updates.installDisabledReason}
              <span class="caption" style:color="var(--text-tertiary)"
                >{updates.installDisabledReason}</span
              >
            {/if}
          </div>
        {/snippet}
        <Banner
          tone="info"
          title={updates.verifiedLabel ?? ''}
          description={updates.verifiedDescription}
          action={updates.onInstall ? installAction : undefined}
        />
      {:else if updates.status === 'installing'}
        <ProgressBar
          indeterminate
          tone="accent"
          label={updates.installingLabel}
        />
        {#if updates.installingLabel}
          <span class="caption" style:color="var(--text-tertiary)"
            >{updates.installingLabel}</span
          >
        {/if}
      {:else if updates.status === 'error'}
        {#snippet retryAction()}
          <Button
            variant="secondary"
            label={updates.retryLabel ?? ''}
            onclick={updates.onRetry}
          />
        {/snippet}
        <Banner
          tone="critical"
          title={updates.errorMessage ?? ''}
          action={updates.onRetry ? retryAction : undefined}
        />
      {/if}
    </div>
  </section>

  <!-- 10. Acerca de y ayuda -->
  <section>
    <h3 class="label section-title">{about.title}</h3>
    <OptionRow label={about.versionRowLabel} description={about.buildLabel}>
      {#snippet control()}
        <span class="body value" style:color="var(--text-secondary)"
          >{about.version}</span
        >
      {/snippet}
    </OptionRow>
    <OptionRow
      label={about.repeatIntroLabel}
      description={about.repeatIntroDescription}
    >
      {#snippet control()}
        <Button
          variant="secondary"
          label={about.repeatIntroLabel}
          onclick={about.onRepeatIntro}
        />
      {/snippet}
    </OptionRow>
    {#each about.links as link (link.label)}
      <OptionRow
        label={link.label}
        description={link.external ? link.externalHint : undefined}
      >
        {#snippet control()}
          <button
            type="button"
            class="link-row-button"
            aria-label={link.label}
            onclick={link.onOpen}
          >
            {#if link.external}
              <svg
                viewBox="0 0 12 12"
                width="11"
                height="11"
                aria-hidden="true"
              >
                <path
                  d="M5 2 H2.5 A0.5 0.5 0 0 0 2 2.5 V9.5 A0.5 0.5 0 0 0 2.5 10 H9.5 A0.5 0.5 0 0 0 10 9.5 V7 M7 2 H10 V5 M10 2 L5.5 6.5"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.3"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            {:else}
              <svg
                viewBox="0 0 12 12"
                width="10"
                height="10"
                aria-hidden="true"
              >
                <path
                  d="M4 2 L8 6 L4 10"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            {/if}
          </button>
        {/snippet}
      </OptionRow>
    {/each}
    {#if about.technicalSummary}
      <div class="status-block">{@render about.technicalSummary()}</div>
    {/if}
    {#if about.advancedLabel && about.detailedLoggingRowLabel}
      {@render advancedToggle('about', about.advancedLabel)}
      {#if advancedOpen.about}
        <div class="advanced">
          <OptionRow
            label={about.detailedLoggingRowLabel}
            description={about.detailedLoggingRowDescription}
          >
            {#snippet control()}
              <Switch
                checked={about.detailedLogging}
                label={about.detailedLoggingLabel ??
                  about.detailedLoggingRowLabel ??
                  ''}
                onchange={about.onDetailedLoggingChange}
              />
            {/snippet}
          </OptionRow>
          {#if about.detailedLoggingUntilLabel}
            <span class="caption effective" style:color="var(--text-tertiary)"
              >{about.detailedLoggingUntilLabel}</span
            >
          {/if}
        </div>
      {/if}
    {/if}
  </section>

  <!-- 11. Zona de riesgo -->
  <section class="risk-zone">
    <h3 class="label section-title" style:color="var(--status-thermal)">
      {riskZone.title}
    </h3>
    <OptionRow
      label={riskZone.resetRowLabel}
      description={riskZone.resetRowDescription}
    >
      {#snippet control()}
        <Button
          variant="secondary"
          label={riskZone.resetButtonLabel}
          disabled={riskZone.resetStatus === 'inProgress'}
          onclick={() => (resetDialogOpen = true)}
        />
      {/snippet}
    </OptionRow>
    {#if riskZone.resetStatus === 'inProgress'}
      <div class="status-block">
        <ProgressBar
          indeterminate
          tone="thermal"
          label={riskZone.resetButtonLabel}
        />
      </div>
    {:else if riskZone.resetStatus === 'error'}
      <div class="status-block">
        <Banner tone="critical" title={riskZone.resetErrorMessage ?? ''} />
      </div>
    {/if}
  </section>

  {#if footer}
    <div class="footer">{@render footer()}</div>
  {/if}
</div>

<Dialog
  bind:open={deleteDialogOpen}
  title={privacy.deleteDialogTitle}
  description={privacy.deleteDialogDescription}
  tone="warning"
>
  {#snippet actions()}
    <Button
      variant="secondary"
      label={privacy.deleteDialogCancelLabel}
      onclick={() => (deleteDialogOpen = false)}
    />
    <Button
      variant="destructive"
      label={privacy.deleteDialogConfirmLabel}
      onclick={confirmDelete}
    />
  {/snippet}
</Dialog>

<Dialog
  bind:open={resetDialogOpen}
  title={riskZone.resetDialogTitle}
  description={riskZone.resetDialogDescription}
  tone="warning"
>
  {#snippet actions()}
    <Button
      variant="secondary"
      label={riskZone.resetDialogCancelLabel}
      onclick={() => (resetDialogOpen = false)}
    />
    <Button
      variant="destructive"
      label={riskZone.resetDialogConfirmLabel}
      onclick={confirmReset}
    />
  {/snippet}
</Dialog>

<style>
  .tw-settings-screen {
    background: transparent;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-6);
    font-family: var(
      --font-sans,
      -apple-system,
      BlinkMacSystemFont,
      'Segoe UI',
      Roboto,
      'Helvetica Neue',
      Arial,
      sans-serif
    );
    container-type: inline-size;
    container-name: tw-settings;
  }
  section {
    display: flex;
    flex-direction: column;
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
  }
  section:nth-child(2) {
    animation-delay: calc(1 * var(--motion-stagger));
  }
  section:nth-child(3) {
    animation-delay: calc(2 * var(--motion-stagger));
  }
  section:nth-child(4) {
    animation-delay: calc(3 * var(--motion-stagger));
  }
  section:nth-child(5) {
    animation-delay: calc(4 * var(--motion-stagger));
  }
  section:nth-child(6) {
    animation-delay: calc(5 * var(--motion-stagger));
  }
  .section-title {
    margin: 0 0 var(--space-2) 0;
  }
  .risk-zone {
    border-color: color-mix(
      in srgb,
      var(--status-thermal) 30%,
      var(--glass-border)
    );
  }
  .advanced {
    animation: tw-rise var(--motion-base) var(--motion-ease-out) both;
  }
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) 0;
  }
  .sensors-complete {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }
  .sensors-complete-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .effective {
    padding: var(--space-1) 0 0;
  }
  .value {
    font-variant-numeric: tabular-nums;
  }
  .disclosure {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: 4px 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    cursor: default;
  }
  .disclosure:hover {
    background: var(--surface-sunken);
    color: var(--text-primary);
  }
  .disclosure:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .chevron {
    display: flex;
    transition: transform 160ms ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .advanced {
    display: flex;
    flex-direction: column;
    margin-top: var(--space-1);
    padding-left: var(--space-4);
    border-left: 2px solid var(--hairline);
  }
  .note {
    padding: var(--space-2) 0;
  }
  .safety {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
  }
  .safety-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .safety-row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .safety-row dt,
  .safety-row dd {
    margin: 0;
  }
  .install-action {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
  }
  .link-row-button {
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    padding: 4px;
    display: flex;
    border-radius: var(--radius-sm);
  }
  .link-row-button:hover {
    background: var(--surface-sunken);
    color: var(--text-primary);
  }
  .link-row-button:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .footer {
    padding-top: var(--space-2);
  }

  /* Compact tier (<700px, matches tokens.BREAKPOINTS.compact): stack
     each OptionRow's label and control instead of forcing the control
     into a shrinking trailing column. Scoped to this component's own
     container width, not the window, so SettingsScreen stays correct
     however it's hosted (full window, a settings pane, a dialog). */
  @container tw-settings (max-width: 699px) {
    .tw-settings-screen {
      padding: var(--space-4);
      gap: var(--space-5);
    }
    .tw-settings-screen :global(.row) {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-2);
    }
    .tw-settings-screen :global(.row .control) {
      width: 100%;
    }
    .tw-settings-screen :global(.tw-select-wrap),
    .tw-settings-screen :global(.tw-segmented) {
      width: 100%;
    }
  }
</style>
