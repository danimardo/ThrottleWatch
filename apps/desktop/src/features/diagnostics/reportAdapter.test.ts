import { describe, expect, it } from 'vitest';
import { toReportImpactView } from './reportAdapter';

describe('report impact adapter', () => {
  it('only exposes a cooling range when the backend declares power_headroom', () => {
    const view = toReportImpactView({
      classification: 'thermal_confirmed',
      coolingPotential: {
        lowPercent: 5,
        highPercent: 15,
        method: 'power_headroom'
      }
    });
    expect(view.coolingPotential?.value).toBe('+5–15 %');
    expect(view.unavailableReason).toBe('');
  });

  it('keeps external guided observation free of an invented percentage', () => {
    const view = toReportImpactView({
      classification: 'thermal_confirmed',
      guidedResult: {
        initialOpsPerSecond: 100,
        sustainedOpsPerSecond: 90,
        relativePercent: null,
        method: 'external_observation'
      }
    });
    expect(view.guidedObservation?.relativePercent).toBeNull();
    expect(view.coolingPotential).toBeUndefined();
  });
});
