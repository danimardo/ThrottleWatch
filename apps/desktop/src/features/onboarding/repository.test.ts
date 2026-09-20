import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeBridge } from '../../test-support/bridge';
import { defaultOnboardingState } from './model';
import { loadOnboardingState, saveOnboardingState } from './repository';

describe('onboarding repository', () => {
  beforeEach(() => {
    const values = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      clear: () => {
        values.clear();
      }
    });
  });

  afterEach(() => vi.unstubAllGlobals());

  it('uses the validated bridge when Tauri is available', async () => {
    const bridge = new FakeBridge();
    const state = { ...defaultOnboardingState(), last_slide: 3 };
    await saveOnboardingState(state, bridge);
    await expect(loadOnboardingState(bridge)).resolves.toMatchObject({
      last_slide: 3
    });
  });

  it('degrades to browser storage without Tauri', async () => {
    const state = { ...defaultOnboardingState(), last_slide: 2 };
    await saveOnboardingState(state);
    await expect(loadOnboardingState()).resolves.toMatchObject({
      last_slide: 2
    });
  });
});
