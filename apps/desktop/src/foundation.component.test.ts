import { describe, expect, it } from 'vitest';

describe('component project', () => {
  it('has a DOM-capable test project', () => {
    expect(document.body).toBeDefined();
  });
});
