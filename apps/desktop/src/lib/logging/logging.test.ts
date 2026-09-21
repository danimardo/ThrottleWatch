import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createLogger, isDetailedLoggingActive } from './index';

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

  it('expires detailed logging at the persisted deadline', () => {
    const deadline = '2026-09-20T12:00:00.000Z';
    const before = Date.parse('2026-09-20T11:59:59.999Z');
    const atDeadline = Date.parse(deadline);

    expect(isDetailedLoggingActive(deadline, before)).toBe(true);
    expect(isDetailedLoggingActive(deadline, atDeadline)).toBe(false);
    expect(isDetailedLoggingActive(null, before)).toBe(false);
  });
});
