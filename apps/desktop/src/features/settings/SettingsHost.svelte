<script lang="ts">
  import { onMount } from 'svelte';
  import SettingsScreen, {
    type SettingsOption,
    type SettingsSensorStatus,
    type SettingsDangerStatus
  } from '../../design-system/components/SettingsScreen.svelte';
  import TechnicalSummary from '../../design-system/components/TechnicalSummary.svelte';
  import LicensesScreen, {
    type LicenseEntry
  } from '../../design-system/components/LicensesScreen.svelte';
  import {
    commandResponseSchemas,
    type CoverageMatrix,
    type PreferencesSnapshot,
    type UpdateState
  } from '../../lib/bridge/schemas';
  import { invokeValidated, listenValidated } from '../../lib/bridge';
  import { blockedReasonKey, toUpdatesSection } from './updates';
  import { accessRequestAction } from '../../lib/access-action';
  import { createTranslator } from '../../lib/i18n';
  import {
    isDetailedLoggingActive,
    setApplicationDetailedLogging
  } from '../../lib/logging';

  interface Props {
    onRepeatIntroduction: () => void;
  }

  let { onRepeatIntroduction }: Props = $props();
  const { t } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );

  let preferences = $state<PreferencesSnapshot | null>(null);
  let coverage = $state<CoverageMatrix | null>(null);
  let storageUsage = $state<{
    database_bytes: number;
    logs_bytes: number;
    total_bytes: number;
    session_count: number;
  } | null>(null);
  let loading = $state(true);
  let updateState = $state<UpdateState | null>(null);
  let installBlockedKey = $state<string | undefined>();
  let technicalSummary = $state('');
  let licenseEntries = $state<LicenseEntry[]>([]);
  let copiedSummary = $state(false);
  let deleteStatus = $state<SettingsDangerStatus>('idle');
  let deleteErrorMessage = $state('');
  let resetStatus = $state<SettingsDangerStatus>('idle');
  let resetErrorMessage = $state('');

  const options = (values: Array<[string, string]>): SettingsOption[] =>
    values.map(([value, label]) => ({ value, label }));

  const preferenceValue = <T,>(key: string, fallback: T): T => {
    const value = preferences?.values[key];
    return (value as T | undefined) ?? fallback;
  };

  let quietPeriod = $derived(
    preferenceValue<{ start: string; end: string } | null>(
      'notifications.quiet_period',
      null
    )
  );

  async function setPreference(key: string, value: unknown): Promise<void> {
    const expected = preferences?.schema_version ?? 1;
    const result = await invokeValidated(
      'set_preference',
      { request: { key, value, expected_schema_version: expected } },
      commandResponseSchemas.set_preference
    );
    if (result.ok) {
      preferences = result.value;
      if (key === 'logging.detailed_until') {
        setApplicationDetailedLogging(
          isDetailedLoggingActive(result.value.values['logging.detailed_until'])
        );
      }
    }
  }

  function setQuietPeriodEnabled(enabled: boolean): void {
    void setPreference(
      'notifications.quiet_period',
      enabled ? (quietPeriod ?? { start: '22', end: '07' }) : null
    );
  }

  function setQuietPeriod(start: string, end: string): void {
    void setPreference('notifications.quiet_period', { start, end });
  }

  async function load(): Promise<void> {
    const [saved, detected, usage, summary, notices] = await Promise.all([
      invokeValidated(
        'get_preferences',
        undefined,
        commandResponseSchemas.get_preferences
      ),
      invokeValidated(
        'get_coverage',
        undefined,
        commandResponseSchemas.get_coverage
      ),
      invokeValidated(
        'get_storage_usage',
        undefined,
        commandResponseSchemas.get_storage_usage
      ),
      invokeValidated(
        'get_technical_summary',
        undefined,
        commandResponseSchemas.get_technical_summary
      ),
      invokeValidated(
        'get_third_party_notices',
        undefined,
        commandResponseSchemas.get_third_party_notices
      )
    ]);
    if (saved.ok) {
      preferences = saved.value;
      setApplicationDetailedLogging(
        isDetailedLoggingActive(saved.value.values['logging.detailed_until'])
      );
    }
    if (detected.ok) coverage = detected.value;
    if (usage.ok) storageUsage = usage.value;
    if (summary.ok) technicalSummary = summary.value.text;
    if (notices.ok) {
      licenseEntries = notices.value.entries.map((entry) => ({
        ...entry,
        version: entry.version ?? undefined
      }));
    }
    loading = false;
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function deleteAllData(): Promise<void> {
    deleteStatus = 'inProgress';
    deleteErrorMessage = '';
    const result = await invokeValidated(
      'delete_monitoring_data',
      { request: { confirmation_token: 'settings-delete-data' } },
      commandResponseSchemas.delete_monitoring_data
    );
    if (result.ok) {
      deleteStatus = result.value.failed.length === 0 ? 'idle' : 'error';
      if (result.value.failed.length > 0) {
        deleteErrorMessage = t('settings.partialOperation');
      }
      await load();
    } else {
      deleteStatus = 'error';
      deleteErrorMessage = t('settings.operationFailed');
    }
  }

  async function resetApplication(): Promise<void> {
    resetStatus = 'inProgress';
    resetErrorMessage = '';
    const result = await invokeValidated(
      'reset_application',
      { request: { confirmation_token: 'settings-reset-application' } },
      commandResponseSchemas.reset_application
    );
    if (result.ok) {
      resetStatus = result.value.failed.length === 0 ? 'idle' : 'error';
      if (result.value.failed.length > 0) {
        resetErrorMessage = t('settings.partialOperation');
      }
      await load();
    } else {
      resetStatus = 'error';
      resetErrorMessage = t('settings.operationFailed');
    }
  }

  async function copyTechnicalSummary(): Promise<void> {
    if (
      !technicalSummary ||
      typeof navigator === 'undefined' ||
      !navigator.clipboard
    )
      return;
    try {
      await navigator.clipboard.writeText(technicalSummary);
      copiedSummary = true;
      window.setTimeout(() => (copiedSummary = false), 1600);
    } catch {
      copiedSummary = false;
    }
  }

  async function openLogsFolder(): Promise<void> {
    await invokeValidated(
      'open_logs_folder',
      undefined,
      commandResponseSchemas.open_logs_folder
    );
  }

  async function openExternal(target: 'help' | 'source'): Promise<void> {
    await invokeValidated(
      'open_external_url',
      { request: { target } },
      commandResponseSchemas.open_external_url
    );
  }

  async function recheckCoverage(): Promise<void> {
    const result = await invokeValidated(
      'recheck_coverage',
      undefined,
      commandResponseSchemas.recheck_coverage
    );
    if (result.ok) coverage = result.value;
  }

  function accessNote(): string | undefined {
    switch (coverage?.advanced_access) {
      case 'installable':
        return t('settings.accessNote.installable');
      case 'upgradable':
        return t('settings.accessNote.upgradable');
      case 'available':
        return t('settings.accessNote.available');
      case 'denied':
        return t('settings.accessNote.denied');
      case 'error':
        return t('settings.accessNote.error');
      default:
        return undefined;
    }
  }

  async function requestAdvancedAccess(): Promise<void> {
    const action = coverage && accessRequestAction(coverage.advanced_access);
    if (!action) return;
    const result = await invokeValidated(
      'request_low_level_access',
      { request: { action } },
      commandResponseSchemas.request_low_level_access
    );
    if (result.ok) await recheckCoverage();
  }

  async function disableAdvancedAccess(): Promise<void> {
    const result = await invokeValidated(
      'disable_advanced_access',
      undefined,
      commandResponseSchemas.disable_advanced_access
    );
    if (result.ok) coverage = result.value;
  }

  async function loadUpdateState(): Promise<void> {
    const result = await invokeValidated(
      'get_update_state',
      undefined,
      commandResponseSchemas.get_update_state
    );
    if (result.ok) updateState = result.value;
  }

  async function updateCommand(
    command: 'check_for_update' | 'download_update' | 'install_update',
    args?: Record<string, unknown>
  ): Promise<void> {
    installBlockedKey = undefined;
    const result = await invokeValidated(
      command,
      args,
      commandResponseSchemas[command]
    );
    if (!result.ok && command === 'install_update') {
      installBlockedKey = blockedReasonKey(result.error.message_key);
    }
  }

  onMount(() => {
    void load();
    void loadUpdateState();
    let stop: (() => void) | undefined;
    void listenValidated('update:state-changed', (value) => {
      updateState = value as UpdateState;
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  });

  const closeActionOptions = options([
    ['exit', t('settings.exit')],
    ['tray', t('settings.tray')]
  ]);
  const languageOptions = options([
    ['system', t('settings.languageSystem')],
    ['es', t('settings.languageSpanish')],
    ['en', t('settings.languageEnglish')]
  ]);
  const themeOptions = options([
    ['system', t('settings.system')],
    ['light', t('settings.light')],
    ['dark', t('settings.dark')]
  ]);
  const motionOptions = options([
    ['system', t('settings.system')],
    ['reduced', t('settings.reduced')],
    ['full', t('settings.full')]
  ]);
  const glassOptions = options([
    ['system', t('settings.system')],
    ['full', t('settings.full')],
    ['reduced', t('settings.reduced')],
    ['off', t('settings.off')]
  ]);
  const samplingOptions = options([
    ['low_power', t('settings.lowPower')],
    ['normal', t('settings.normal')],
    ['diagnostic', t('settings.diagnostic')]
  ]);
  const batteryOptions = options([
    ['keep', t('settings.keep')],
    ['low_power', t('settings.lowPower')],
    ['pause', t('settings.pause')]
  ]);
  const retentionOptions = options([
    ['session', t('settings.sessionOnly')],
    ['1d', '1 d'],
    ['7d', '7 d'],
    ['30d', '30 d']
  ]);
  const durationOptions = options([
    ['short', t('settings.short')],
    ['standard', t('settings.standard')],
    ['long', t('settings.long')]
  ]);

  let sensorStatus: SettingsSensorStatus = $derived(
    coverage === null
      ? 'detecting'
      : coverage.tier === 'A'
        ? 'complete'
        : 'partial'
  );
</script>

{#if loading}
  <div class="settings-loading" role="status">{t('settings.loading')}</div>
{:else}
  <div class="settings-host">
    <SettingsScreen
      general={{
        title: t('settings.generalTitle'),
        closeActionRowLabel: t('settings.closeAction'),
        closeActionRowDescription: t('settings.closeActionDescription'),
        closeActionLabel: t('settings.closeAction'),
        closeAction: preferenceValue('lifecycle.close_action', 'unset'),
        closeActionOptions,
        closeActionUndecidedText: t('settings.closeActionUndecided'),
        onCloseActionChange: (value) =>
          void setPreference('lifecycle.close_action', value),
        startOnLogin: preferenceValue('startup.enabled', false),
        startOnLoginLabel: t('settings.startOnLogin'),
        startOnLoginDescription: t('settings.startOnLoginDescription'),
        onStartOnLoginChange: (checked) =>
          void setPreference('startup.enabled', checked),
        startHiddenLabel: t('settings.startHidden'),
        startHiddenDescription: t('settings.startHiddenDescription'),
        startHiddenInTray:
          preferenceValue<string>('startup.mode', 'window') === 'tray',
        onStartHiddenInTrayChange: (checked) =>
          void setPreference('startup.mode', checked ? 'tray' : 'window'),
        startHiddenDisabled: !preferenceValue('tray.monitoring_enabled', false),
        startHiddenDisabledReason: t('settings.trayRequired')
      }}
      language={{
        title: t('settings.languageTitle'),
        rowLabel: t('settings.language'),
        selectLabel: t('settings.language'),
        value: preferenceValue('locale.mode', 'system'),
        options: languageOptions,
        onChange: (value) => void setPreference('locale.mode', value)
      }}
      appearance={{
        title: t('settings.appearanceTitle'),
        rowLabel: t('settings.theme'),
        segmentedLabel: t('settings.theme'),
        theme: preferenceValue('appearance.theme', 'system'),
        options: themeOptions,
        onThemeChange: (value) => void setPreference('appearance.theme', value),
        motionRowLabel: t('settings.motion'),
        motionLabel: t('settings.motion'),
        motion: preferenceValue('appearance.motion', 'system'),
        motionOptions,
        onMotionChange: (value) =>
          void setPreference('appearance.motion', value),
        glassRowLabel: t('settings.glass'),
        glassLabel: t('settings.glass'),
        glass: preferenceValue('appearance.glass', 'system'),
        glassOptions,
        onGlassChange: (value) => void setPreference('appearance.glass', value)
      }}
      monitoring={{
        title: t('settings.monitoringTitle'),
        modeRowLabel: t('settings.profile'),
        modeLabel: t('settings.profile'),
        mode: preferenceValue('sampling.profile', 'normal'),
        modeOptions: samplingOptions,
        onModeChange: (value) => void setPreference('sampling.profile', value),
        onBatteryRowLabel: t('settings.onBattery'),
        onBatteryLabel: t('settings.onBattery'),
        onBattery: preferenceValue('sampling.on_battery', 'keep'),
        onBatteryOptions: batteryOptions,
        onOnBatteryChange: (value) =>
          void setPreference('sampling.on_battery', value),
        advancedLabel: t('settings.advanced'),
        intervalRowLabel: t('settings.interval'),
        intervalSelectLabel: t('settings.interval'),
        samplingInterval: preferenceValue('sampling.profile', 'normal'),
        samplingIntervalOptions: samplingOptions,
        onSamplingIntervalChange: (value) =>
          void setPreference('sampling.profile', value),
        perCoreHistoryLabel: t('settings.perCoreHistory'),
        perCoreHistory: preferenceValue('sampling.per_core_history', false),
        onPerCoreHistoryChange: (checked) =>
          void setPreference('sampling.per_core_history', checked)
      }}
      tray={{
        title: t('settings.trayTitle'),
        backgroundLabel: t('settings.backgroundMonitoring'),
        backgroundMonitoring: preferenceValue('tray.monitoring_enabled', false),
        onBackgroundMonitoringChange: (checked) =>
          void setPreference('tray.monitoring_enabled', checked),
        notificationsLabel: t('settings.notifications'),
        notifications: preferenceValue('notifications.enabled', false),
        onNotificationsChange: (checked) =>
          void setPreference('notifications.enabled', checked),
        advancedLabel: t('settings.advanced'),
        quietPeriodLabel: t('settings.quietPeriod'),
        quietPeriodEnabled: quietPeriod !== null,
        onQuietPeriodEnabledChange: setQuietPeriodEnabled,
        quietPeriodStartLabel: t('settings.quietStart'),
        quietPeriodEndLabel: t('settings.quietEnd'),
        quietPeriodStart: quietPeriod?.start ?? '22',
        quietPeriodEnd: quietPeriod?.end ?? '07',
        quietPeriodOptions: options([
          ['22', '22:00'],
          ['07', '07:00']
        ]),
        onQuietPeriodChange: setQuietPeriod
      }}
      privacy={{
        title: t('settings.privacyTitle'),
        anonymizeExportsLabel: t('settings.anonymizeExports'),
        anonymizeExports: preferenceValue('privacy.anonymize_exports', true),
        onAnonymizeExportsChange: (checked) =>
          void setPreference('privacy.anonymize_exports', checked),
        retentionRowLabel: t('settings.retention'),
        retentionSelectLabel: t('settings.retention'),
        dataRetention: preferenceValue('history.retention', '7d'),
        dataRetentionOptions: retentionOptions,
        onDataRetentionChange: (value) =>
          void setPreference('history.retention', value),
        storageRowLabel: t('settings.storage'),
        storageValue: storageUsage
          ? formatBytes(storageUsage.total_bytes)
          : t('settings.notAvailable'),
        storageDescription: storageUsage
          ? `${formatBytes(storageUsage.database_bytes)} + ${formatBytes(storageUsage.logs_bytes)} · ${storageUsage.session_count}`
          : undefined,
        exportRowLabel: t('settings.export'),
        exportButtonLabel: t('settings.export'),
        onExport: () =>
          window.dispatchEvent(new CustomEvent('throttlewatch:request-export')),
        deleteRowLabel: t('settings.deleteData'),
        deleteButtonLabel: t('settings.deleteData'),
        deleteStatus,
        deleteErrorMessage,
        onDeleteAllData: () => void deleteAllData(),
        deleteDialogTitle: t('settings.deleteData'),
        deleteDialogDescription: t('settings.notAvailable'),
        deleteDialogCancelLabel: t('settings.cancel'),
        deleteDialogConfirmLabel: t('settings.deleteData')
      }}
      sensors={{
        title: t('settings.sensorsTitle'),
        rowLabel: t('settings.coverage'),
        status: sensorStatus,
        detectingLabel: t('settings.loading'),
        advancedAccess: coverage?.advanced_access,
        advancedAccessNote: accessNote(),
        requestAccessLabel:
          coverage?.advanced_access === 'installable'
            ? t('settings.installAccess')
            : coverage?.advanced_access === 'upgradable'
              ? t('settings.upgradeAccess')
              : t('settings.repairAccess'),
        onRequestAdvancedAccess: () => void requestAdvancedAccess(),
        accessRetryLabel: t('settings.repairAccess'),
        onAccessRetry: () => void requestAdvancedAccess(),
        accessHelpLabel: t('settings.accessHelp'),
        onAccessHelp: () => void openExternal('help'),
        disableAccessLabel: t('settings.disableAccess'),
        onDisableAdvancedAccess: () => void disableAdvancedAccess(),
        recheckLabel: t('settings.recheck'),
        onRecheck: () => void recheckCoverage()
      }}
      diagnostics={{
        title: t('settings.diagnosticsTitle'),
        durationRowLabel: t('settings.duration'),
        durationLabel: t('settings.duration'),
        duration: preferenceValue('guided.duration', 'standard'),
        durationOptions,
        onDurationChange: (value) =>
          void setPreference('guided.duration', value),
        requireAcLabel: t('settings.requireAc'),
        requireAc: preferenceValue('guided.require_ac', false),
        onRequireAcChange: (checked) =>
          void setPreference('guided.require_ac', checked),
        notifyLabel: t('settings.notifyOnFinish'),
        notifyOnFinish: preferenceValue('guided.notify_on_finish', false),
        onNotifyOnFinishChange: (checked) =>
          void setPreference('guided.notify_on_finish', checked),
        notifyDisabled: !preferenceValue('notifications.enabled', false),
        notifyDisabledReason: t('settings.notificationsRequired'),
        safetyLimitsTitle: t('settings.safetyLimits'),
        safetyLimits: []
      }}
      updates={toUpdatesSection({
        state: updateState ?? {
          state: 'idle',
          enabled: preferenceValue('updates.enabled', false),
          current_version: '0.1.0',
          last_check: null,
          version: null,
          notes: null,
          downloaded: null,
          total: null,
          error_code: null,
          recoverable: null
        },
        t,
        formatDate: (value) =>
          new Date(value).toLocaleString(
            typeof navigator === 'undefined' ? 'en-US' : navigator.language
          ),
        blockedReasonKey: installBlockedKey,
        onAutoCheckChange: (checked) =>
          void setPreference('updates.enabled', checked),
        onCheckNow: () =>
          void updateCommand('check_for_update', { request: { manual: true } }),
        onDownload: () => void updateCommand('download_update'),
        onInstall: () => void updateCommand('install_update')
      })}
      about={{
        title: t('settings.aboutTitle'),
        versionRowLabel: t('settings.version'),
        version: updateState?.current_version ?? '0.1.0',
        repeatIntroLabel: t('settings.repeatIntroduction'),
        onRepeatIntro: onRepeatIntroduction,
        links: [
          {
            label: t('settings.openLogsFolder'),
            onOpen: () => void openLogsFolder()
          },
          {
            label: t('settings.helpLink'),
            onOpen: () => void openExternal('help'),
            external: true,
            externalHint: t('settings.opensBrowser')
          },
          {
            label: t('settings.sourceLink'),
            onOpen: () => void openExternal('source'),
            external: true,
            externalHint: t('settings.opensBrowser')
          }
        ],
        technicalSummary: technicalSummarySnippet,
        advancedLabel: t('settings.advanced'),
        detailedLoggingRowLabel: t('settings.detailedLogging'),
        detailedLogging:
          preferenceValue('logging.detailed_until', null) !== null,
        detailedLoggingUntilLabel: preferenceValue(
          'logging.detailed_until',
          null
        )
          ? t('settings.detailedLoggingActive')
          : t('settings.detailedLoggingInactive'),
        onDetailedLoggingChange: (checked) =>
          void setPreference('logging.detailed_until', checked)
      }}
      riskZone={{
        title: t('settings.riskZone'),
        resetRowLabel: t('settings.reset'),
        resetButtonLabel: t('settings.reset'),
        resetStatus,
        resetErrorMessage,
        onReset: () => void resetApplication(),
        resetDialogTitle: t('settings.reset'),
        resetDialogDescription: t('settings.notAvailable'),
        resetDialogCancelLabel: t('settings.cancel'),
        resetDialogConfirmLabel: t('settings.reset')
      }}
      footer={licensesFooter}
    />
  </div>
{/if}

{#snippet technicalSummarySnippet()}
  <TechnicalSummary
    title={t('settings.technicalSummary')}
    note={t('settings.technicalSummaryNote')}
    text={technicalSummary || t('settings.notAvailable')}
    copyLabel={t('settings.copySummary')}
    copiedLabel={t('settings.copiedSummary')}
    copied={copiedSummary}
    onCopy={() => void copyTechnicalSummary()}
    regionLabel={t('settings.technicalSummary')}
  />
{/snippet}

{#snippet licensesFooter()}
  <LicensesScreen
    title={t('settings.licenses')}
    intro={t('settings.licensesIntro')}
    entries={licenseEntries}
  />
{/snippet}

<style>
  .settings-host {
    min-height: 100%;
    overflow: auto;
  }

  .settings-loading {
    display: grid;
    min-height: 20rem;
    place-items: center;
    color: var(--text-secondary);
  }
</style>
