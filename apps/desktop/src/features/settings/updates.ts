import type { UpdateState } from '../../lib/bridge/schemas';

// Structural copies of `UpdatesStatus` / `UpdatesSection` (SettingsScreen.svelte):
// a `.ts` file cannot import types from a `.svelte` module under the linter, and `svelte-check`
// verifies at the use site in `SettingsHost` that these stay assignable to the real props.
export type UpdatesStatus =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'verified'
  | 'installing'
  | 'error';

export interface UpdatesSection {
  title: string;
  versionRowLabel: string;
  currentVersion: string;
  autoCheckLabel: string;
  autoCheck: boolean;
  onAutoCheckChange: (checked: boolean) => void;
  lastCheckLabel?: string;
  lastCheckValue?: string;
  status: UpdatesStatus;
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

const STATUS: Record<UpdateState['state'], UpdatesStatus> = {
  idle: 'idle',
  checking: 'checking',
  up_to_date: 'upToDate',
  available: 'available',
  downloading: 'downloading',
  verified: 'verified',
  installing: 'installing',
  error: 'error'
};

const ERROR_KEYS: Record<string, string> = {
  'update.network': 'settings.updateErrorNetwork',
  'update.signature_invalid': 'settings.updateErrorSignature',
  'update.no_trusted_key': 'settings.updateErrorNoKey',
  'update.install_failed': 'settings.updateErrorInstall'
};

const BLOCKED = ['guided_test', 'export', 'import', 'data_operation'];

export function statusOf(state: UpdateState['state']): UpdatesStatus {
  return STATUS[state];
}

export function errorKey(code: string | null): string {
  return (code && ERROR_KEYS[code]) || 'settings.updateErrorGeneric';
}

/** The catalog key for `update.blocked.<reason>` (the message key the backend answers with). */
export function blockedReasonKey(messageKey: string): string | undefined {
  const reason = messageKey.startsWith('update.blocked.')
    ? messageKey.slice('update.blocked.'.length)
    : undefined;
  return reason && BLOCKED.includes(reason)
    ? `settings.updateBlocked.${reason}`
    : undefined;
}

interface Input {
  state: UpdateState;
  t: (key: string) => string;
  formatDate: (value: string) => string;
  /** Catalog key of why an install was refused, from the last `install_update` answer. */
  blockedReasonKey?: string;
  onAutoCheckChange: (checked: boolean) => void;
  onCheckNow: () => void;
  onDownload: () => void;
  onInstall: () => void;
}

/** The `updates` section of `SettingsScreen` for the updater state the backend reported. */
export function toUpdatesSection(input: Input): UpdatesSection {
  const { state, t } = input;
  const percent =
    state.downloaded !== null && state.total
      ? Math.min(100, Math.round((state.downloaded / state.total) * 100))
      : 0;
  return {
    title: t('settings.updatesTitle'),
    versionRowLabel: t('settings.version'),
    currentVersion: state.current_version,
    autoCheckLabel: t('settings.autoUpdates'),
    autoCheck: state.enabled,
    onAutoCheckChange: input.onAutoCheckChange,
    lastCheckLabel: t('settings.updateLastCheck'),
    lastCheckValue: state.last_check
      ? input.formatDate(state.last_check)
      : t('settings.updateNever'),
    status: statusOf(state.state),
    checkNowLabel: t('settings.checkNow'),
    onCheckNow: input.onCheckNow,
    checkNowDisabled: !state.enabled,
    checkNowDisabledReason: t('settings.updateCheckNowDisabled'),
    checkingLabel: t('settings.updateChecking'),
    upToDateLabel: t('settings.updateUpToDate'),
    availableVersion: t('settings.updateAvailable').replace(
      '{version}',
      state.version ?? ''
    ),
    availableDescription:
      state.notes || t('settings.updateAvailableDescription'),
    downloadLabel: t('settings.updateDownload'),
    onDownload: state.state === 'available' ? input.onDownload : undefined,
    downloadPercent: percent,
    downloadingLabel: t('settings.updateDownloading'),
    verifiedLabel: t('settings.updateVerified'),
    verifiedDescription: t('settings.updateVerifiedDescription'),
    installLabel: t('settings.updateInstall'),
    onInstall: state.state === 'verified' ? input.onInstall : undefined,
    installDisabled: input.blockedReasonKey !== undefined,
    installDisabledReason: input.blockedReasonKey
      ? t(input.blockedReasonKey)
      : undefined,
    installingLabel: t('settings.updateInstalling'),
    errorMessage: t(errorKey(state.error_code)),
    retryLabel: t('settings.updateRetry'),
    onRetry: state.recoverable ? input.onCheckNow : undefined
  };
}
