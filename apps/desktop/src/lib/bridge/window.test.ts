import { describe, expect, it } from 'vitest';
import { createWindowAdapter } from './window';

describe('window bridge', () => {
  it('degrades to inert controls outside Tauri', async () => {
    const adapter = createWindowAdapter();
    await expect(adapter.minimize()).resolves.toBeUndefined();
    await expect(adapter.toggleMaximize()).resolves.toBeUndefined();
    await expect(adapter.close()).resolves.toBeUndefined();
    await expect(adapter.isMaximized()).resolves.toBe(false);
  });
});
