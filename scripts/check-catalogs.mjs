import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const read = (name) => JSON.parse(fs.readFileSync(path.join(root, name), 'utf8'));
const flatten = (value, prefix = '') => Object.entries(value).flatMap(([key, child]) => {
  const current = prefix ? `${prefix}.${key}` : key;
  if (typeof child === 'string') return [current];
  if (child && typeof child === 'object' && !Array.isArray(child)) return flatten(child, current);
  throw new Error(`Invalid catalog value at ${current}`);
});

const es = flatten(read('apps/desktop/src/lib/i18n/locales/es.json')).sort();
const en = flatten(read('apps/desktop/src/lib/i18n/locales/en.json')).sort();
if (JSON.stringify(es) !== JSON.stringify(en)) {
  throw new Error('Spanish and English catalogs do not have the same keys');
}
if (new Set(es).size !== es.length || es.length === 0) {
  throw new Error('Catalog keys must be non-empty and unique');
}
process.stdout.write(`catalogs: ${es.length} keys, 0 mismatches\n`);
