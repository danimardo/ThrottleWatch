import { describe, expect, it } from 'vitest';
import { validateEnvelope } from './index';

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
