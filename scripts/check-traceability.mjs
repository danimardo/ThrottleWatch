import { readFile } from 'node:fs/promises';

const path = 'specs/001-cpu-thermal-diagnostics/traceability.md';
const content = await readFile(path, 'utf8');
if (!content.includes('| Requisito | Cobertura | Estado |')) {
  throw new Error('traceability.md no contiene la tabla canónica');
}
const source = await Promise.all([
  readFile('specs/001-cpu-thermal-diagnostics/spec.md', 'utf8'),
  readFile('specs/001-cpu-thermal-diagnostics/plan.md', 'utf8'),
  readFile('specs/001-cpu-thermal-diagnostics/research.md', 'utf8')
]);
const requirements = new Set(
  source.join('\n').match(/\b(?:FR|NFR|SC)-\d{3}\b/g) ?? []
);
const covered = new Set();
for (const match of content.matchAll(/\b(FR|NFR|SC)-(\d{3})–\1-(\d{3})\b/g)) {
  const [, prefix, first, last] = match;
  for (let number = Number(first); number <= Number(last); number += 1) {
    covered.add(`${prefix}-${String(number).padStart(3, '0')}`);
  }
}
for (const requirement of requirements) {
  if (!covered.has(requirement))
    throw new Error(`Requisito sin entrada de trazabilidad: ${requirement}`);
}
