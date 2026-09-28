import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  REDACTED,
  createLogger,
  effectiveFrontendLevel,
  installGlobalErrorLogging,
  isDetailedLoggingActive,
  redactFields,
  type FrontendLogEvent,
  type Logger
} from './index';

function recorder() {
  const events: FrontendLogEvent[] = [];
  const send = vi.fn((batch: readonly FrontendLogEvent[]) => {
    events.push(...batch);
    return Promise.resolve();
  });
  return { events, send };
}

describe('frontend logging', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('filters info in production until detailed logging is enabled', () => {
    const { send } = recorder();
    const instance = createLogger('analysis', { send, now: () => 1_000 });

    instance.logger.info('UI_INFO', 'hidden');
    expect(send).not.toHaveBeenCalled();
    instance.setDetailed(true);
    instance.logger.info('UI_INFO', 'visible');
    expect(send).toHaveBeenCalledOnce();
  });

  it('caps events at sixty per minute', () => {
    const { send } = recorder();
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

  it('sends code, message and target intact, with only allow-listed fields in the clear', () => {
    const { events, send } = recorder();
    const instance = createLogger('guided', { send, now: () => 1_000 });

    instance.logger.warn('GUIDED_ACTION_FAILED', 'guided start failed', {
      reason: 'guided.error',
      user_name: 'Ada'
    });
    expect(events).toEqual([
      {
        level: 'warn',
        code: 'GUIDED_ACTION_FAILED',
        target: 'guided',
        msg: 'guided start failed',
        fields: { reason: 'guided.error', user_name: REDACTED }
      }
    ]);
  });

  it('never drops an event whose code is invalid: it becomes LOG_EVENT_WITHOUT_CODE', () => {
    const { events, send } = recorder();
    const instance = createLogger('analysis', { send, now: () => 1_000 });

    instance.logger.debug('BAD CODE', 'lost detail');
    expect(events).toHaveLength(1);
    expect(events[0]?.level).toBe('error');
    expect(events[0]?.code).toBe('LOG_EVENT_WITHOUT_CODE');
    expect(events[0]?.msg).toBe('debug lost detail');
  });

  it('truncates a message longer than the backend accepts instead of losing the batch', () => {
    const { events, send } = recorder();
    const instance = createLogger('analysis', { send, now: () => 1_000 });

    instance.logger.error('UI_LONG', 'x'.repeat(400));
    expect(events[0]?.msg.length).toBe(256);
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

describe('effective frontend level (constitution XVII)', () => {
  it('development follows PUBLIC_LOG_LEVEL, debug by default', () => {
    expect(
      effectiveFrontendLevel({
        development: true,
        publicLevel: 'debug',
        detailed: false
      })
    ).toBe('debug');
    expect(
      effectiveFrontendLevel({
        development: true,
        publicLevel: 'trace',
        detailed: false
      })
    ).toBe('trace');
    expect(
      effectiveFrontendLevel({
        development: true,
        publicLevel: 'warn',
        detailed: false
      })
    ).toBe('warn');
  });

  it('in development, detailed logging can only make it more verbose', () => {
    expect(
      effectiveFrontendLevel({
        development: true,
        publicLevel: 'warn',
        detailed: true
      })
    ).toBe('debug');
    expect(
      effectiveFrontendLevel({
        development: true,
        publicLevel: 'trace',
        detailed: true
      })
    ).toBe('trace');
  });

  it('production ignores PUBLIC_LOG_LEVEL and never reaches trace', () => {
    expect(
      effectiveFrontendLevel({
        development: false,
        publicLevel: 'trace',
        detailed: false
      })
    ).toBe('warn');
    expect(
      effectiveFrontendLevel({
        development: false,
        publicLevel: 'trace',
        detailed: true
      })
    ).toBe('debug');
  });

  it('a development logger at trace forwards trace events', () => {
    const { send } = recorder();
    const instance = createLogger('analysis', {
      send,
      development: true,
      publicLevel: 'trace',
      now: () => 1_000
    });
    instance.logger.trace('UI_TRACE', 'per-sample detail');
    expect(send).toHaveBeenCalledOnce();
    expect(instance.level()).toBe('trace');
  });
});

describe('redaction', () => {
  it('keeps allow-listed categorical fields and replaces everything else', () => {
    expect(
      redactFields({
        status: 'aborted',
        count: 3,
        path: 'C:\\Users\\ada\\file.json',
        reason: 'x'.repeat(200)
      })
    ).toEqual({
      status: 'aborted',
      count: 3,
      path: REDACTED,
      reason: REDACTED
    });
    expect(redactFields(undefined)).toBeUndefined();
  });
});

describe('global error logging', () => {
  function fakeLogger(): { logger: Logger; errors: unknown[][] } {
    const errors: unknown[][] = [];
    const noop = (): void => undefined;
    return {
      errors,
      logger: {
        trace: noop,
        debug: noop,
        info: noop,
        warn: noop,
        error: (...args: unknown[]) => {
          errors.push(args);
        }
      }
    };
  }

  it('logs an uncaught error and an unhandled rejection as UI_UNHANDLED_ERROR', () => {
    const target = new EventTarget();
    const { logger, errors } = fakeLogger();
    const remove = installGlobalErrorLogging(target, logger);

    const error = new Event('error');
    Object.defineProperty(error, 'error', { value: new TypeError('boom') });
    target.dispatchEvent(error);
    const rejection = new Event('unhandledrejection');
    Object.defineProperty(rejection, 'reason', {
      value: new RangeError('late')
    });
    target.dispatchEvent(rejection);

    expect(errors).toEqual([
      ['UI_UNHANDLED_ERROR', 'TypeError: boom', { state: 'error' }],
      [
        'UI_UNHANDLED_ERROR',
        'RangeError: late',
        { state: 'unhandledrejection' }
      ]
    ]);

    remove();
    target.dispatchEvent(error);
    expect(errors).toHaveLength(2);
  });
});
