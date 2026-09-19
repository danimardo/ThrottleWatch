import { describe, expect, it } from 'vitest';
import { liveSnapshot } from '../../test-support/bridge';
import { fromLiveSnapshot } from './model';

describe('dashboard snapshot adapter', () => {
  it('keeps missing readings as null and maps degraded collector states', () => {
    const snapshot = fromLiveSnapshot(liveSnapshot({
      temperature_c: null,
      active_clock_mhz: null,
      collector_state: 'degraded',
      coverage: { tier: 'B', confidence_ceiling: 'medium', advanced_access: 'installable' }
    }));
    expect(snapshot.temperatureC).toBeNull();
    expect(snapshot.activeClockMhz).toBeNull();
    expect(snapshot.collectorState).toBe('stale');
    expect(snapshot.coverage).toBe('B');
  });

  it('represents complete, basic and disconnected coverage without fabricating values', () => {
    for (const tier of ['A', 'B', 'C'] as const) {
      const snapshot = fromLiveSnapshot(liveSnapshot({
        collector_state: tier === 'C' ? 'stopped' : 'running',
        coverage: { tier, confidence_ceiling: tier === 'A' ? 'high' : tier === 'B' ? 'medium' : 'low', advanced_access: tier === 'A' ? 'not_needed' : 'installable' }
      }));
      expect(snapshot.coverage).toBe(tier);
    }
    const disconnected = fromLiveSnapshot(liveSnapshot({ collector_state: 'failed', temperature_c: null }));
    expect(disconnected.collectorState).toBe('disconnected');
    expect(disconnected.temperatureC).toBeNull();
  });
});
