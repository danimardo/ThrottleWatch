import { cp, mkdir, readdir, readFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const sourceRoot = join(root, 'design');
const targetRoot = join(root, 'apps', 'desktop', 'src', 'design-system');
const directories = [
  'components',
  'icons',
  'illustrations',
  'lib',
  'tokens',
  'brand'
];
const checkOnly = process.argv.includes('--check');

async function main() {
  const differences = [];
  for (const directory of directories) {
    const source = join(sourceRoot, directory);
    const target = join(targetRoot, directory);
    if (!checkOnly) {
      await mkdir(target, { recursive: true });
      await cp(source, target, { recursive: true, force: true });
      continue;
    }
    await compareTree(source, target, differences);
  }

  if (differences.length > 0) {
    throw new Error(
      `La copia del sistema de diseño difiere en:\n${differences.join('\n')}`
    );
  }
}

async function compareTree(source, target, differences) {
  let sourceEntries;
  try {
    sourceEntries = await readdir(source, { withFileTypes: true });
  } catch {
    differences.push(relative(root, source));
    return;
  }
  for (const entry of sourceEntries) {
    const sourcePath = join(source, entry.name);
    const targetPath = join(target, entry.name);
    if (entry.isDirectory()) {
      await compareTree(sourcePath, targetPath, differences);
      continue;
    }
    try {
      const [sourceBytes, targetBytes] = await Promise.all([
        readFile(sourcePath),
        readFile(targetPath)
      ]);
      if (!sourceBytes.equals(targetBytes)) {
        differences.push(relative(root, targetPath));
      }
    } catch {
      differences.push(relative(root, targetPath));
    }
  }
  try {
    const targetEntries = await readdir(target, { withFileTypes: true });
    const sourceNames = new Set(sourceEntries.map((entry) => entry.name));
    for (const entry of targetEntries) {
      if (!sourceNames.has(entry.name)) {
        differences.push(relative(root, join(target, entry.name)));
      }
    }
  } catch {
    differences.push(relative(root, target));
  }
}

await main();
