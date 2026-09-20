import {
  invokeValidated,
  type BridgeTransport
} from '../../lib/bridge';
import {
  commandResponseSchemas,
  onboardingStateSchema,
  type OnboardingState
} from '../../lib/bridge/schemas';
import { defaultOnboardingState } from './model';

const BROWSER_STORAGE_KEY = 'throttlewatch.onboarding-state';

function readBrowserState(): OnboardingState {
  if (typeof localStorage === 'undefined') return defaultOnboardingState();
  try {
    const raw = localStorage.getItem(BROWSER_STORAGE_KEY);
    if (raw === null) return defaultOnboardingState();
    const parsed = onboardingStateSchema.safeParse(JSON.parse(raw));
    return parsed.success ? parsed.data : defaultOnboardingState();
  } catch {
    return defaultOnboardingState();
  }
}

function writeBrowserState(state: OnboardingState): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(BROWSER_STORAGE_KEY, JSON.stringify(state));
}

export async function loadOnboardingState(
  transport?: BridgeTransport
): Promise<OnboardingState> {
  const result = await invokeValidated(
    'get_onboarding_state',
    undefined,
    commandResponseSchemas.get_onboarding_state,
    transport
  );
  if (result.ok) return result.value;
  return readBrowserState();
}

export async function saveOnboardingState(
  state: OnboardingState,
  transport?: BridgeTransport
): Promise<void> {
  const result = await invokeValidated(
    'set_onboarding_state',
    state,
    onboardingStateSchema,
    transport
  );
  if (!result.ok) writeBrowserState(state);
}
