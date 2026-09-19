import type { Classification } from '../../design-system/lib/classification';

export interface CoolingPotentialView {
  value: string;
  method: string;
}

export interface GuidedObservationView {
  initialOpsPerSecond: number;
  sustainedOpsPerSecond: number;
  relativePercent: number | null;
  method: 'external_observation';
}

export interface ReportImpactView {
  coolingPotential?: CoolingPotentialView;
  guidedObservation?: GuidedObservationView;
  unavailableReason: string;
}

export interface ReportBackendView {
  classification: Classification;
  coolingPotential?: { lowPercent: number; highPercent: number; method: 'power_headroom' };
  guidedResult?: GuidedObservationView;
}

export function toReportImpactView(report: ReportBackendView): ReportImpactView {
  return {
    coolingPotential: report.coolingPotential
      ? {
          value: `+${String(report.coolingPotential.lowPercent)}–${String(report.coolingPotential.highPercent)} %`,
          method: report.coolingPotential.method
        }
      : undefined,
    guidedObservation: report.guidedResult,
    unavailableReason: report.guidedResult
      ? 'The guided run uses external observation and has no performance percentage.'
      : report.coolingPotential
        ? ''
        : 'The inputs required by an approved method are unavailable.'
  };
}
