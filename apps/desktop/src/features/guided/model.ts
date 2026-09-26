import type {
  GuidedPhase as BridgeGuidedPhase,
  GuidedPreflight
} from '../../lib/bridge/schemas';
export type DiagnosticPhase =
  | 'intro'
  | 'preflight'
  | 'ready'
  | 'rest'
  | 'warming'
  | 'steadyLoad'
  | 'recovery'
  | 'cancelling'
  | 'cancelled'
  | 'safetyStop'
  | 'sensorLost'
  | 'error'
  | 'result';

export interface GuidedCheck {
  label: string;
  status: 'checking' | 'ok' | 'failed';
  detail?: string;
}

const phaseMap: Record<BridgeGuidedPhase['phase'], DiagnosticPhase> = {
  preflight: 'preflight',
  ready: 'ready',
  rest: 'rest',
  warming: 'warming',
  steady_load: 'steadyLoad',
  recovery: 'recovery',
  cancelling: 'cancelling',
  cancelled: 'cancelled',
  safety_stop: 'safetyStop',
  sensor_lost: 'sensorLost',
  error: 'error',
  result: 'result'
};

export function toDiagnosticPhase(
  phase: BridgeGuidedPhase['phase']
): DiagnosticPhase {
  return phaseMap[phase];
}

export function preflightChecks(
  value: GuidedPreflight,
  t: (key: string) => string
): GuidedCheck[] {
  return [
    {
      label: t('guided.checks.sensors'),
      status: value.sensors ? 'ok' : 'failed'
    },
    {
      label: t('guided.checks.power'),
      status: value.ac_power || !value.require_ac ? 'ok' : 'failed'
    },
    {
      label: t('guided.checks.profile'),
      status: value.profile ? 'ok' : 'failed'
    },
    {
      label: t('guided.checks.disk'),
      status: value.disk_space ? 'ok' : 'failed'
    },
    {
      label: t('guided.checks.generator'),
      status: value.generator ? 'ok' : 'failed'
    }
  ];
}

/** A person on battery when the test requires AC cannot start it: say so above the checks. */
export function batteryStateOf(
  value: GuidedPreflight | null
): 'ok' | 'blocked' {
  return value !== null && value.require_ac && !value.ac_power
    ? 'blocked'
    : 'ok';
}

export function percentRemaining(
  elapsedMs: number,
  remainingMs: number | null
): number | undefined {
  if (remainingMs === null) return undefined;
  const total = elapsedMs + remainingMs;
  return total <= 0 ? 0 : Math.min(100, Math.max(0, (elapsedMs / total) * 100));
}
