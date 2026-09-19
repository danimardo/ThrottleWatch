import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createLogger } from './index';

describe('frontend logging', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('filters info until detailed logging is enabled', () => {
    const send = vi.fn(() => Promise.resolve());
    const clock = vi.fn(() => 1_000);
    const instance = createLogger('analysis', { send, now: clock });

    instance.logger.info('UI_INFO', 'hidden');
    expect(send).not.toHaveBeenCalled();
    instance.setDetailed(true);
    instance.logger.info('UI_INFO', 'visible');
    expect(send).toHaveBeenCalledOnce();
  });

  it('caps events at sixty per minute', () => {
    const send = vi.fn(() => Promise.resolve());
    const instance = createLogger('analysis', {
      send,
      detailed: true,
      now: () => 1_000
    });

    for (let index = 0; index < 61; index += 1) {
      instance.logger.warn('UI_WARNING', `event-${String(index)}`);
    }
    expect(send).toHaveBeenCalledTimes(60);
  });
});
