import { readFile } from 'node:fs/promises';

const labels = JSON.parse(await readFile(new URL('../corpus/labels-v1.json', import.meta.url), 'utf8'));
const required = new Set([
  'intel-hybrid-thermal',
  'intel-turbo-end',
  'portable-dptf',
  'desktop-open-limits',
  'amd-zen4-thermal',
  'few-cores-game',
  'hot-start',
  'eco-qos'
]);

if (labels.version !== 1 || labels.cases.length !== required.size) {
  throw new Error('corpus version or case count is invalid');
}
for (const item of labels.cases) {
  if (!required.has(item.id) || !item.source || !item.expected || item.reasons.length === 0 || item.levels.length === 0) {
    throw new Error(`invalid corpus label: ${item.id}`);
  }
}
process.stdout.write(`corpus v${labels.version}: ${labels.cases.length} labeled scenarios\n`);
