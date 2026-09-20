import { describe, expect, it } from 'vitest';
import { resolveTheme } from './appearance';

describe('appearance', () => {
  it('follows the operating system only for the system preference', () => {
    expect(resolveTheme('system', true)).toBe('light');
    expect(resolveTheme('system', false)).toBe('dark');
    expect(resolveTheme('light', false)).toBe('light');
    expect(resolveTheme('dark', true)).toBe('dark');
  });
});
