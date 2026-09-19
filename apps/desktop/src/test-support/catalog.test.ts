import { describe, expect, it } from 'vitest';
import { createCatalogQuery } from './catalog';

describe('catalog test query', () => {
  const t = createCatalogQuery({
    common: { continue: 'Continuar' },
    screen: { title: 'Ahora' }
  });

  it('resolves dotted keys', () => {
    expect(t('common.continue')).toBe('Continuar');
    expect(t('screen.title')).toBe('Ahora');
  });

  it('fails loudly for a missing key', () => {
    expect(() => t('screen.missing')).toThrow(
      'Missing catalog key: screen.missing'
    );
  });
});
