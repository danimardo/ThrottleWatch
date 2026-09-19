import { describe, expect, it } from 'vitest';
import { ipcEnvelopeSchema } from './schemas';

const fixtureFiles = import.meta.glob<string>(
  '../../../../../packages/contracts/fixtures/*.json',
  { eager: true, import: 'default', query: '?raw' }
);

describe('IPC contract fixtures', () => {
  for (const path of Object.keys(fixtureFiles).sort()) {
    const filename = path.substring(path.lastIndexOf('/') + 1);

    it(`${filename} matches the shared acceptance convention`, () => {
      const raw = fixtureFiles[path];
      if (raw === undefined) throw new Error(`Fixture is missing: ${path}`);
      const value: unknown = JSON.parse(raw);
      const accepted = ipcEnvelopeSchema.safeParse(value).success;
      const expected = !filename.startsWith('invalid-');

      expect(accepted).toBe(expected);
    });
  }
});
