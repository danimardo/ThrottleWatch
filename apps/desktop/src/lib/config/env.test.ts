import { describe, expect, it } from 'vitest';
import { parsePublicEnv } from './env';

describe('public environment', () => {
  it('uses debug by default, as development does (constitution XVII)', () => {
    expect(parsePublicEnv({}).PUBLIC_LOG_LEVEL).toBe('debug');
  });

  it('accepts every level, trace included', () => {
    for (const level of ['trace', 'debug', 'info', 'warn', 'error']) {
      expect(parsePublicEnv({ PUBLIC_LOG_LEVEL: level }).PUBLIC_LOG_LEVEL).toBe(
        level
      );
    }
  });

  it('rejects unsupported log levels', () => {
    expect(() => parsePublicEnv({ PUBLIC_LOG_LEVEL: 'verbose' })).toThrow();
  });
});
