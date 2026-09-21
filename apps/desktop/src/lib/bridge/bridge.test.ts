import { describe, expect, it } from 'vitest';
import {
  createBridge,
  fakeBridgeAllowed,
  invokeValidated,
  listenValidated,
  parseEvent,
  validateEnvelope
} from './index';
import { z } from 'zod';
import { FakeBridge, liveSnapshot } from '../../test-support/bridge';
import {
  coverageMatrixSchema,
  liveSnapshotSchema,
  trayStateSchema
} from './schemas';

const validEnvelope = {
  protocol_version: 1,
  session_nonce: 'fixture-session-nonce-1',
  sequence: 0,
  timestamp_utc: '2026-09-18T10:00:00.000Z',
  type: 'hello',
  payload: { app_version: '0.1.0', supported_protocols: [1] }
};

describe('fake bridge gate', () => {
  it('allows the injected fake bridge only in the e2e and test build modes', () => {
    expect(fakeBridgeAllowed('e2e')).toBe(true);
    expect(fakeBridgeAllowed('test')).toBe(true);
    expect(fakeBridgeAllowed('production')).toBe(false);
    expect(fakeBridgeAllowed('development')).toBe(false);
    expect(fakeBridgeAllowed('')).toBe(false);
  });
});

describe('bridge envelope validation', () => {
  it('accepts the canonical envelope shape', () => {
    expect(validateEnvelope(validEnvelope)).toMatchObject({ ok: true });
  });

  it('returns a structured error for invalid envelopes', () => {
    const result = validateEnvelope({ ...validEnvelope, protocol_version: 2 });
    expect(result).toMatchObject({
      ok: false,
      error: { code: 'BRIDGE_VALIDATION_FAILED' }
    });
  });
});

describe('bridge outside Tauri', () => {
  it('returns a transport error instead of invoking Tauri', async () => {
    const result = await invokeValidated(
      'get_live_snapshot',
      undefined,
      z.unknown()
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: 'transport',
        code: 'TAURI_UNAVAILABLE',
        message_key: 'bridge.tauri_unavailable'
      }
    });
  });

  it('returns a no-op unsubscriber instead of registering a Tauri listener', async () => {
    const unlisten = await listenValidated(
      'telemetry:snapshot',
      () => undefined
    );

    unlisten();
  });
});

describe('injectable bridge transport', () => {
  it('accepts a coverage transition with its stable reason', () => {
    const result = parseEvent('coverage:changed', {
      tier: 'B',
      confidence_ceiling: 'medium',
      advanced_access: 'denied',
      rows: [],
      conclusion_key: 'coverage.conclusion.b',
      from_tier: 'A',
      to_tier: 'B',
      reason: 'provider_error'
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.from_tier).toBe('A');
      expect(result.value.to_tier).toBe('B');
      expect(result.value.reason).toBe('provider_error');
    }
  });

  it('invokes and validates a fake command response', async () => {
    const bridge = createBridge(new FakeBridge());
    const result = await bridge.invokeValidated(
      'get_live_snapshot',
      undefined,
      liveSnapshotSchema
    );

    expect(result).toEqual({ ok: true, value: liveSnapshot() });
  });

  it('delivers only schema-valid fake events', async () => {
    const fake = new FakeBridge();
    const bridge = createBridge(fake);
    const values: unknown[] = [];
    const stop = await bridge.listenValidated('collector:state', (value) =>
      values.push(value)
    );

    fake.emit('collector:state', { state: 'running' });
    fake.emit('collector:state', { state: 'not-a-state' });
    stop();

    expect(values).toEqual([{ state: 'running' }]);
  });

  it('pauses the fake collector through the same nested request as Tauri', async () => {
    const fake = new FakeBridge();
    const bridge = createBridge(fake);
    const result = await bridge.invokeValidated(
      'set_tray_paused',
      { request: { paused: true } },
      trayStateSchema
    );

    expect(result).toEqual({
      ok: true,
      value: { icon: 'unknown', paused: true }
    });
  });
});

describe('advanced access vocabulary', () => {
  it('accepts every state the host derives, including an older PawnIO', () => {
    for (const state of [
      'not_needed',
      'available',
      'installable',
      'upgradable',
      'denied',
      'error'
    ]) {
      const parsed = coverageMatrixSchema.safeParse({
        tier: 'B',
        confidence_ceiling: 'medium',
        advanced_access: state,
        rows: [],
        conclusion_key: 'coverage.conclusion.b'
      });
      expect(parsed.success, state).toBe(true);
    }
  });

  it('rejects a state outside the closed vocabulary', () => {
    const parsed = coverageMatrixSchema.safeParse({
      tier: 'B',
      confidence_ceiling: 'medium',
      advanced_access: 'reinstallable',
      rows: [],
      conclusion_key: 'coverage.conclusion.b'
    });
    expect(parsed.success).toBe(false);
  });
});
