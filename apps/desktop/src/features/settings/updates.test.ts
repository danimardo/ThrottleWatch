import { describe, expect, it } from 'vitest';
import { updateStateSchema } from '../../lib/bridge/schemas';
import {
  blockedReasonKey,
  errorKey,
  statusOf,
  toUpdatesSection
} from './updates';

const t = (key: string): string =>
  key === 'settings.updateAvailable' ? 'Versión {version}' : `«${key}»`;

function state(overrides: Record<string, unknown> = {}) {
  return updateStateSchema.parse({
    state: 'idle',
    enabled: true,
    current_version: '0.1.0',
    last_check: null,
    version: null,
    notes: null,
    downloaded: null,
    total: null,
    error_code: null,
    recoverable: null,
    ...overrides
  });
}

const handlers = {
  onAutoCheckChange: () => undefined,
  onCheckNow: () => undefined,
  onDownload: () => undefined,
  onInstall: () => undefined
};

function section(overrides: Record<string, unknown> = {}, blocked?: string) {
  return toUpdatesSection({
    state: state(overrides),
    t,
    formatDate: (value) => `fecha:${value}`,
    blockedReasonKey: blocked,
    ...handlers
  });
}

describe('updates settings section', () => {
  it('maps every backend state to the design system status', () => {
    expect(statusOf('idle')).toBe('idle');
    expect(statusOf('checking')).toBe('checking');
    expect(statusOf('up_to_date')).toBe('upToDate');
    expect(statusOf('available')).toBe('available');
    expect(statusOf('downloading')).toBe('downloading');
    expect(statusOf('verified')).toBe('verified');
    expect(statusOf('installing')).toBe('installing');
    expect(statusOf('error')).toBe('error');
  });

  it('cannot check while the updater is off and says why', () => {
    const off = section({ enabled: false });
    expect(off.checkNowDisabled).toBe(true);
    expect(off.checkNowDisabledReason).toBe(
      '«settings.updateCheckNowDisabled»'
    );
    expect(section().checkNowDisabled).toBe(false);
  });

  it('shows Download only for an available version and Install only once verified', () => {
    const available = section({ state: 'available', version: '9.9.9' });
    expect(available.status).toBe('available');
    expect(available.availableVersion).toContain('9.9.9');
    expect(available.onDownload).toBe(handlers.onDownload);
    expect(available.onInstall).toBeUndefined();

    const verified = section({ state: 'verified', version: '9.9.9' });
    expect(verified.onInstall).toBe(handlers.onInstall);
    expect(verified.onDownload).toBeUndefined();
    expect(verified.installDisabled).toBe(false);
  });

  it('computes the download percentage from the bytes the backend reports', () => {
    expect(
      section({ state: 'downloading', downloaded: 25, total: 100 })
        .downloadPercent
    ).toBe(25);
    expect(
      section({ state: 'downloading', downloaded: 5, total: null })
        .downloadPercent
    ).toBe(0);
    expect(
      section({ state: 'downloading', downloaded: 500, total: 100 })
        .downloadPercent
    ).toBe(100);
  });

  it('blocks the install with the reason the backend gave', () => {
    const blocked = section(
      { state: 'verified', version: '9.9.9' },
      'settings.updateBlocked.guided_test'
    );
    expect(blocked.installDisabled).toBe(true);
    expect(blocked.installDisabledReason).toBe(
      '«settings.updateBlocked.guided_test»'
    );
  });

  it('turns an error code into a catalog message and offers a retry only when it can help', () => {
    const network = section({
      state: 'error',
      error_code: 'update.network',
      recoverable: true
    });
    expect(network.errorMessage).toBe('«settings.updateErrorNetwork»');
    expect(network.onRetry).toBe(handlers.onCheckNow);

    const signature = section({
      state: 'error',
      error_code: 'update.signature_invalid',
      recoverable: false
    });
    expect(signature.errorMessage).toBe('«settings.updateErrorSignature»');
    expect(signature.onRetry).toBeUndefined();
    expect(errorKey('anything.else')).toBe('settings.updateErrorGeneric');
    expect(errorKey(null)).toBe('settings.updateErrorGeneric');
  });

  it('shows the last check when there is one and the installed version always', () => {
    const checked = section({ last_check: '2026-09-21T08:00:00Z' });
    expect(checked.currentVersion).toBe('0.1.0');
    expect(checked.lastCheckValue).toBe('fecha:2026-09-21T08:00:00Z');
    expect(section().lastCheckValue).toBe('«settings.updateNever»');
  });

  it('only recognises the blocker keys the backend defines', () => {
    expect(blockedReasonKey('update.blocked.export')).toBe(
      'settings.updateBlocked.export'
    );
    expect(blockedReasonKey('update.blocked.nonsense')).toBeUndefined();
    expect(blockedReasonKey('bridge.unknown_error')).toBeUndefined();
  });
});
