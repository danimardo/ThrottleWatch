import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const read = (name) =>
  JSON.parse(fs.readFileSync(path.join(root, name), 'utf8'));
const flatten = (value, prefix = '') =>
  Object.entries(value).flatMap(([key, child]) => {
    const current = prefix ? `${prefix}.${key}` : key;
    if (typeof child === 'string') return [current];
    if (child && typeof child === 'object' && !Array.isArray(child))
      return flatten(child, current);
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

const uiFiles = [
  'apps/desktop/src/features/guided/GuidedDiagnostic.svelte',
  'apps/desktop/src/features/analysis/Analysis.svelte',
  'apps/desktop/src/features/cpu/CpuOverview.svelte',
  'apps/desktop/src/shell/AppShell.svelte'
];
const userFacingProperties =
  '(?:title|description|label|placeholder|aria-label|tooltipLabel|idleHint|stepperLabel|' +
  'loadingLabel|noHistoryTitle|noHistoryDescription|coverageTitle|coverageDescription|' +
  'tableLabel|tableFilterLabel|tableFilterPlaceholder|tableEmptyFilterMessage|' +
  'formatLabel|anonymizeLabel|anonymizeDescription|includedTitle|excludedTitle|sizeLabel|' +
  'cancelLabel|confirmLabel|fileNameLabel)';
const staticAttribute = new RegExp(
  `\\b${userFacingProperties}\\s*=\\s*["']([^"']+)["']`,
  'g'
);
const staticText = />\s*([A-Za-zÁÉÍÓÚáéíóúÑñ¿¡][^<{]*)\s*</g;
const staticObjectProperty = new RegExp(
  `\\b${userFacingProperties}\\s*:\\s*(['"])(.*?)\\1`,
  'g'
);
const uiViolations = [];
for (const file of uiFiles) {
  const source = fs.readFileSync(path.join(root, file), 'utf8');
  const markup = source
    .replace(/<script[\s\S]*?<\/script>/g, '')
    .replace(/<style[\s\S]*?<\/style>/g, '');
  for (const match of markup.matchAll(staticAttribute))
    uiViolations.push(`${file}: static ${match[0]}`);
  for (const match of markup.matchAll(staticText)) {
    if (!/[{}();=]/.test(match[1]))
      uiViolations.push(`${file}: static text ${match[1].trim()}`);
  }
  for (const match of markup.matchAll(staticObjectProperty))
    uiViolations.push(`${file}: static ${match[0]}`);
}
if (uiViolations.length > 0) {
  throw new Error(`UI literals must use catalogs:\n${uiViolations.join('\n')}`);
}
process.stdout.write(
  `ui-catalogs: ${uiFiles.length} screens, 0 static literals\n`
);
