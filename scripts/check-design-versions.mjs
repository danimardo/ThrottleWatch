import { readFile } from 'node:fs/promises';

const app = JSON.parse(await readFile('apps/desktop/package.json', 'utf8'));
const harness = JSON.parse(await readFile('design/harness/package.json', 'utf8'));
const mockup = JSON.parse(await readFile('design/mockup/package.json', 'utf8'));

const names = [
  'svelte',
  'vite',
  'typescript',
  '@sveltejs/vite-plugin-svelte',
  'svelte-check',
  '@tsconfig/svelte',
  '@types/node'
];
const harnessOnlyNames = ['vitest', 'jsdom', '@testing-library/svelte'];
const appVersions = { ...app.dependencies, ...app.devDependencies };
const mismatches = [];

for (const name of names) {
  const expected = appVersions[name];
  for (const [label, packageJson] of [
    ['design/harness', harness],
    ['design/mockup', mockup]
  ]) {
    const actual = packageJson.devDependencies?.[name];
    if (actual !== expected || /[~^*<>]/.test(actual ?? '')) {
      mismatches.push(`${label}:${name} expected=${expected} actual=${actual}`);
    }
  }
}

for (const name of harnessOnlyNames) {
  const expected = appVersions[name];
  const actual = harness.devDependencies?.[name];
  if (actual !== expected || /[~^*<>]/.test(actual ?? '')) {
    mismatches.push(`design/harness:${name} expected=${expected} actual=${actual}`);
  }
}

if (mismatches.length > 0) {
  console.error(mismatches.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`design versions: ${names.length + harnessOnlyNames.length} dependencies aligned`);
}
