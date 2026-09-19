import { describe, expect, it } from 'vitest';
import { E2E_PROFILE_NAMES, E2E_PROFILES } from '../../e2e/profile-data';

describe('E2E profiles', () => {
  it('defines the same six isolated scenarios for frontend and app', () => {
    expect(E2E_PROFILES.map((profile) => profile.name)).toEqual(
      E2E_PROFILE_NAMES
    );
    expect(
      E2E_PROFILES.every((profile) => Object.keys(profile.appSeed).length > 0)
    ).toBe(true);
  });
});
