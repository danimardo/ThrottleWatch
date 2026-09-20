import { describe, expect, it } from 'vitest';
import { collectorStateEventSchema } from '../../lib/bridge/schemas';
import { catalogs } from '../../lib/i18n';
import { COLLECTOR_LABEL_KEYS } from './model';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function lookup(catalog: unknown, key: string): unknown {
  return key
    .split('.')
    .reduce<unknown>(
      (node, part) => (isRecord(node) ? node[part] : undefined),
      catalog
    );
}

describe('collector state labels', () => {
  const states = collectorStateEventSchema.shape.state.options;

  it('has a label key for every state the backend can report', () => {
    for (const state of states) {
      expect(COLLECTOR_LABEL_KEYS[state], state).toBeTypeOf('string');
    }
  });

  it('has a translation in both catalogs for every one of those keys', () => {
    for (const key of Object.values(COLLECTOR_LABEL_KEYS)) {
      for (const locale of ['es', 'en'] as const) {
        expect(lookup(catalogs[locale], key), `${locale}: ${key}`).toBeTypeOf(
          'string'
        );
      }
    }
  });
});
