import { describe, expect, it } from 'vitest';
import { parseEvent } from '../lib/bridge';
import {
  FakeBridge,
  coverage,
  liveSnapshot,
  preferences,
  report,
  session
} from './bridge';

describe('FakeBridge', () => {
  it('creates schema-validated live and coverage fixtures', () => {
    const bridge = new FakeBridge(liveSnapshot(), coverage());
    expect(bridge.getLiveSnapshot().coverage.tier).toBe('A');
    expect(bridge.getCoverage().rows.length).toBeGreaterThan(0);
  });

  it('validates event payloads at the bridge boundary', () => {
    expect(parseEvent('telemetry:snapshot', liveSnapshot()).ok).toBe(true);
    expect(parseEvent('collector:state', { state: 'not-a-state' }).ok).toBe(
      false
    );
  });

  it('provides schema-validated builders for component scenarios', () => {
    expect(report().schema_version).toBe(1);
    expect(session({ status: 'completed' }).status).toBe('completed');
    expect(preferences({ language: 'en' }).language).toBe('en');
  });
});
