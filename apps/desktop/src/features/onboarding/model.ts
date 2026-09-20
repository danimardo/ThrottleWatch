import type { OnboardingState } from '../../lib/bridge/schemas';

export const ONBOARDING_FLOW_VERSION = 1;
export const ONBOARDING_SLIDE_COUNT = 5;
export const CURRENT_NOTICE_VERSION = 1;

export function defaultOnboardingState(): OnboardingState {
  return {
    flow_version: ONBOARDING_FLOW_VERSION,
    last_slide: 1,
    status: 'pending',
    completed_at: null,
    last_seen_notice_version: 0
  };
}

export function initialStepFor(state: OnboardingState): number {
  if (state.status !== 'pending') return 0;
  return Math.max(
    0,
    Math.min(ONBOARDING_SLIDE_COUNT - 1, state.last_slide - 1)
  );
}

export function advanceToSlide(
  state: OnboardingState,
  zeroBasedStep: number
): OnboardingState {
  return {
    ...state,
    flow_version: ONBOARDING_FLOW_VERSION,
    last_slide: Math.max(
      1,
      Math.min(ONBOARDING_SLIDE_COUNT, zeroBasedStep + 1)
    ),
    status: 'pending',
    completed_at: null
  };
}

export function resolveOnboarding(
  state: OnboardingState,
  status: 'completed' | 'skipped',
  completedAt: string
): OnboardingState {
  return {
    ...state,
    flow_version: ONBOARDING_FLOW_VERSION,
    status,
    completed_at: completedAt
  };
}

export function repeatOnboarding(state: OnboardingState): OnboardingState {
  return {
    ...state,
    flow_version: ONBOARDING_FLOW_VERSION,
    last_slide: 1,
    completed_at: null,
    status: 'pending'
  };
}

export function markNoticesSeen(
  state: OnboardingState,
  version = CURRENT_NOTICE_VERSION
): OnboardingState {
  return {
    ...state,
    last_seen_notice_version: Math.max(state.last_seen_notice_version, version)
  };
}

export function shouldShowNotices(state: OnboardingState): boolean {
  return state.last_seen_notice_version < CURRENT_NOTICE_VERSION;
}
