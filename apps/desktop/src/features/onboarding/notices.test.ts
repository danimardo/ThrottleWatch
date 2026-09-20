import { describe, expect, it } from 'vitest';
import { CURRENT_NOTICE_VERSION } from './model';
import { NOTICE_DEFINITIONS, pendingNoticeDefinitions } from './notices';

describe('versioned notices', () => {
  it('keeps notices independent from resolved onboarding state', () => {
    expect(pendingNoticeDefinitions(0)).toEqual(NOTICE_DEFINITIONS);
    expect(pendingNoticeDefinitions(CURRENT_NOTICE_VERSION)).toEqual([]);
  });

  it('does not show an older notice again after a newer watermark', () => {
    expect(pendingNoticeDefinitions(CURRENT_NOTICE_VERSION + 1)).toEqual([]);
  });
});
