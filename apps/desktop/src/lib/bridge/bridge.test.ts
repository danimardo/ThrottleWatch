import { describe, expect, it } from 'vitest';
import { invokeValidated, listenValidated, validateEnvelope } from './index';
import { z } from 'zod';

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
