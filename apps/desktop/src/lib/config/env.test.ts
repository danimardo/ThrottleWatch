import { describe, expect, it } from 'vitest';
import { parsePublicEnv } from './env';

describe('public environment', () => {
  it('uses info by default', () => {
    expect(parsePublicEnv({}).PUBLIC_LOG_LEVEL).toBe('info');
  });

  it('rejects unsupported log levels', () => {
    expect(() => parsePublicEnv({ PUBLIC_LOG_LEVEL: 'trace' })).toThrow();
  });
});
