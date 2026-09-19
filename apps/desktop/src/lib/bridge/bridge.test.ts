import { describe, expect, it } from 'vitest';
import {
  createBridge,
  invokeValidated,
  listenValidated,
  validateEnvelope
} from './index';
import { z } from 'zod';
import { FakeBridge, liveSnapshot } from '../../test-support/bridge';
import { liveSnapshotSchema } from './schemas';

const validEnvelope = {
  protocol_version: 1,
  session_nonce: 'fixture-session-nonce-1',
  sequence: 0,
  timestamp_utc: '2026-09-18T10:00:00.000Z',
  type: 'hello',
  payload: { app_version: '0.1.0', supported_protocols: [1] }
};

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
});
