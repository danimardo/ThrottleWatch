import { describe, expect, it } from 'vitest';
import { liveSnapshot } from '../../test-support/bridge';
import { coverageRowView, fromLiveSnapshot } from './model';

describe('dashboard snapshot adapter', () => {
  it('keeps missing readings as null and maps degraded collector states', () => {
    const snapshot = fromLiveSnapshot(
      liveSnapshot({
        temperature_c: null,
        active_clock_mhz: null,
        collector_state: 'degraded',
        coverage: {
          tier: 'B',
          confidence_ceiling: 'medium',
          advanced_access: 'installable'
        }
      })
    );
    expect(snapshot.temperatureC).toBeNull();
    expect(snapshot.activeClockMhz).toBeNull();
    expect(snapshot.collectorState).toBe('stale');
    expect(snapshot.coverage).toBe('B');
  });

  it('represents complete, basic and disconnected coverage without fabricating values', () => {
    for (const tier of ['A', 'B', 'C'] as const) {
      const snapshot = fromLiveSnapshot(
        liveSnapshot({
          collector_state: tier === 'C' ? 'stopped' : 'running',
          coverage: {
            tier,
            confidence_ceiling:
              tier === 'A' ? 'high' : tier === 'B' ? 'medium' : 'low',
            advanced_access: tier === 'A' ? 'not_needed' : 'installable'
          }
        })
      );
      expect(snapshot.coverage).toBe(tier);
    }
    const disconnected = fromLiveSnapshot(
      liveSnapshot({ collector_state: 'failed', temperature_c: null })
    );
    expect(disconnected.collectorState).toBe('disconnected');
    expect(disconnected.temperatureC).toBeNull();
  });
});

describe('coverage row view', () => {
  const t = (key: string): string => `«${key}»`;
  const base = {
    id: 'active_clock',
    label: 'active_clock',
    available: true,
    quality: 'substitute',
    quality_label: 'Sustituta',
    source_label: 'LibreHardwareMonitor',
    reason_key: null
  };

  it('names the magnitude, quality and source from the catalog, not from the backend', () => {
    const view = coverageRowView(t, base);
    expect(view.label).toBe('«dashboard.coverageRow.active_clock»');
    expect(view.qualityLabel).toBe('«dashboard.coverageQuality.substitute»');
    expect(view.sourceLabel).toBe('«dashboard.coverageSource.librehardwaremonitor»');
    expect(view.quality).toBe('substitute');
  });

  it('never prints a raw catalog key or an untranslated word for an unknown row', () => {
    const view = coverageRowView(t, {
      ...base,
      id: 'something_new',
      available: false,
      quality: 'unexpected',
      source_label: 'Some new source',
      reason_key: 'coverage.not_in_the_catalog'
    });
    expect(view.label).toBe('«dashboard.coverageRow.unknown»');
    expect(view.quality).toBe('direct');
    expect(view.sourceLabel).toBe('Some new source');
    expect(view.reasonLabel).toBe('«dashboard.unavailable»');
  });
});
