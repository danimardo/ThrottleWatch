import {
  commandResponseSchemas,
  type GuidedPhase
} from '../../lib/bridge/schemas';
import { invokeValidated } from '../../lib/bridge';

type SessionListener = (phase: GuidedPhase | null) => void;

let currentPhase: GuidedPhase | null = null;
const listeners = new Set<SessionListener>();

function isRunning(phase: GuidedPhase | null): boolean {
  return (
    phase !== null &&
    [
      'ready',
      'rest',
      'warming',
      'steady_load',
      'recovery',
      'cancelling'
    ].includes(phase.phase)
  );
}

export function subscribeGuidedSession(listener: SessionListener): () => void {
  listeners.add(listener);
  listener(currentPhase);
  return () => listeners.delete(listener);
}

export function publishGuidedPhase(phase: GuidedPhase | null): void {
  currentPhase = phase;
  listeners.forEach((listener) => {
    listener(currentPhase);
  });
}

export function guidedSessionIsRunning(): boolean {
  return isRunning(currentPhase);
}

export async function stopGuidedSession(): Promise<boolean> {
  const result = await invokeValidated(
    'stop_guided',
    undefined,
    commandResponseSchemas.stop_guided
  );
  if (!result.ok) return false;
  publishGuidedPhase(result.value);
  return true;
}
