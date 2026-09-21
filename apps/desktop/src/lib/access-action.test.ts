import { describe, expect, it } from 'vitest';
import { accessRequestAction } from './access-action';

describe('accessRequestAction', () => {
  it('maps every access state that offers an action to the matching request', () => {
    expect(accessRequestAction('installable')).toBe('install');
    expect(accessRequestAction('upgradable')).toBe('upgrade');
    expect(accessRequestAction('error')).toBe('repair');
    expect(accessRequestAction('denied')).toBe('repair');
  });

  it('offers nothing when advanced access is available or not needed', () => {
    expect(accessRequestAction('available')).toBeUndefined();
    expect(accessRequestAction('not_needed')).toBeUndefined();
  });
});
