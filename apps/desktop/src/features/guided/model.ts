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
  t: (key: string) => string,
  requireAc: boolean
): GuidedCheck[] {
  return [
    {
      label: t('guided.checks.sensors'),
      status: value.sensors ? 'ok' : 'failed'
    },
    {
      label: t('guided.checks.power'),
      status: value.ac_power || !requireAc ? 'ok' : 'failed'
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

/**
 * On battery the test cannot start when the person asked for AC (`guided.require_ac`), and is only
 * warned about possible power limits when they did not.
 */
export function batteryStateOf(
  value: GuidedPreflight | null,
  requireAc: boolean
): 'ok' | 'warning' | 'blocked' {
  if (value === null || value.ac_power) return 'ok';
  return requireAc ? 'blocked' : 'warning';
}

export function percentRemaining(
  elapsedMs: number,
  remainingMs: number | null
): number | undefined {
  if (remainingMs === null) return undefined;
  const total = elapsedMs + remainingMs;
  return total <= 0 ? 0 : Math.min(100, Math.max(0, (elapsedMs / total) * 100));
}
