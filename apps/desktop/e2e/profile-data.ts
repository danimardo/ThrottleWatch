export const E2E_PROFILE_NAMES = [
  'fresh-install',
  'onboarding-midway',
  'ready',
  'tray-enabled',
  'with-history',
  'updates-on'
] as const;

export type E2eProfileName = (typeof E2E_PROFILE_NAMES)[number];

export type E2eProfile = {
  readonly name: E2eProfileName;
  readonly frontendScenario:
    'empty' | 'onboarding' | 'ready' | 'tray' | 'history' | 'updates';
  readonly appSeed: Readonly<Record<string, unknown>>;
};

export const E2E_PROFILES: readonly E2eProfile[] = [
  {
    name: 'fresh-install',
    frontendScenario: 'empty',
    appSeed: { onboarding: 'pending' }
  },
  {
    name: 'onboarding-midway',
    frontendScenario: 'onboarding',
    appSeed: { onboarding: 'slide-3' }
  },
  {
    name: 'ready',
    frontendScenario: 'ready',
    appSeed: { onboarding: 'completed', collector: 'running' }
  },
  {
    name: 'tray-enabled',
    frontendScenario: 'tray',
    appSeed: { onboarding: 'completed', tray: true }
  },
  {
    name: 'with-history',
    frontendScenario: 'history',
    appSeed: { onboarding: 'completed', sessions: 2 }
  },
  {
    name: 'updates-on',
    frontendScenario: 'updates',
    appSeed: { onboarding: 'completed', updates: true }
  }
];
