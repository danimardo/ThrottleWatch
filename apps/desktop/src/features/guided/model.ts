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

export function preflightChecks(value: GuidedPreflight): GuidedCheck[] {
  return [
    { label: 'Sensores', status: value.sensors ? 'ok' : 'failed' },
    {
      label: 'Alimentación',
      status: value.ac_power || !value.require_ac ? 'ok' : 'failed'
    },
    { label: 'Perfil', status: value.profile ? 'ok' : 'failed' },
    { label: 'Espacio de disco', status: value.disk_space ? 'ok' : 'failed' },
    { label: 'Generador', status: value.generator ? 'ok' : 'failed' }
  ];
}

export function percentRemaining(
  elapsedMs: number,
  remainingMs: number | null
): number | undefined {
  if (remainingMs === null) return undefined;
  const total = elapsedMs + remainingMs;
  return total <= 0 ? 0 : Math.min(100, Math.max(0, (elapsedMs / total) * 100));
}
