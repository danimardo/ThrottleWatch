import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { E2E_PROFILES } from './profile-data';
import type { E2eProfileName } from './profile-data';

export { E2E_PROFILE_NAMES, E2E_PROFILES } from './profile-data';
export type { E2eProfile, E2eProfileName } from './profile-data';

export async function createIsolatedProfile(
  name: E2eProfileName
): Promise<string> {
  const profile = E2E_PROFILES.find((candidate) => candidate.name === name);
  if (!profile) throw new Error(`Unknown E2E profile: ${name}`);
  const directory = await fs.mkdtemp(
    path.join(os.tmpdir(), `throttlewatch-${name}-`)
  );
  await fs.mkdir(path.join(directory, 'app-data'), { recursive: true });
  await fs.writeFile(
    path.join(directory, 'app-data', 'seed.json'),
    `${JSON.stringify(profile.appSeed, null, 2)}\n`,
    'utf8'
  );
  return directory;
}
