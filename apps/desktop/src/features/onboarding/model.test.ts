import { describe, expect, it } from 'vitest';
import {
  advanceToSlide,
  defaultOnboardingState,
  initialStepFor,
  markNoticesSeen,
  repeatOnboarding,
  resolveOnboarding,
  shouldShowNotices
} from './model';

describe('onboarding state machine', () => {
  it('resumes pending onboarding at the persisted slide', () => {
    expect(initialStepFor({ ...defaultOnboardingState(), last_slide: 4 })).toBe(3);
  });

  it('does not resume a resolved onboarding flow', () => {
    const state = resolveOnboarding(defaultOnboardingState(), 'skipped', '2026-09-19T12:00:00Z');
    expect(initialStepFor(state)).toBe(0);
  });

  it('persists navigation without resolving the flow', () => {
    expect(advanceToSlide(defaultOnboardingState(), 4)).toMatchObject({
      last_slide: 5,
      status: 'pending',
      completed_at: null
    });
  });

  it('repetition resets progress but preserves the resolved state history fields', () => {
    const state = repeatOnboarding(
      resolveOnboarding(defaultOnboardingState(), 'completed', '2026-09-19T12:00:00Z')
    );
    expect(state).toMatchObject({ last_slide: 1, status: 'pending', completed_at: null });
  });

  it('shows each notice version once and never lowers the watermark', () => {
    const state = defaultOnboardingState();
    expect(shouldShowNotices(state)).toBe(true);
    const seen = markNoticesSeen(state);
    expect(shouldShowNotices(seen)).toBe(false);
    expect(markNoticesSeen(seen, 0).last_seen_notice_version).toBe(1);
  });
});
