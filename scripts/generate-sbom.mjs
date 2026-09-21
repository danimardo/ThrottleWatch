import { readFile, writeFile } from 'node:fs/promises';

const root = new URL('../', import.meta.url);
const readJson = async (relative) => {
  const bytes = await readFile(new URL(relative, root));
  const encoding = bytes[0] === 0xff && bytes[1] === 0xfe ? 'utf16le' : 'utf8';
  return JSON.parse(bytes.toString(encoding).replace(/^\uFEFF/u, ''));
};
const components = [];

const npm = await readJson('docs/licenses/npm.json');
for (const entries of Object.values(npm)) {
  for (const entry of entries) {
    for (const version of entry.versions ?? []) {
      components.push({
        type: 'library',
        supplier: { name: 'npm' },
        name: entry.name,
        version,
        licenses: [{ license: { id: entry.license ?? 'NOASSERTION' } }]
      });
    }
  }
}

const nuget = await readJson('docs/licenses/nuget.json');
for (const entry of nuget) {
  components.push({
    type: 'library',
    supplier: { name: 'NuGet' },
    name: entry.PackageId,
    version: entry.PackageVersion,
    licenses: [{ license: { id: entry.License ?? 'NOASSERTION' } }]
  });
}

const lock = await readFile(
  new URL('apps/desktop/src-tauri/Cargo.lock', root),
  'utf8'
);
const packageBlocks = lock.split(/\n\[\[package\]\]\n/u).slice(1);
for (const block of packageBlocks) {
  const name = /^name = "([^"]+)"/mu.exec(block)?.[1];
  const version = /^version = "([^"]+)"/mu.exec(block)?.[1];
  if (name && version)
    components.push({
      type: 'library',
      supplier: { name: 'crates.io' },
      name,
      version
    });
}

components.sort((left, right) =>
  `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`)
);
const document = {
  bomFormat: 'CycloneDX',
  specVersion: '1.5',
  serialNumber: 'urn:uuid:00000000-0000-4000-8000-000000000001',
  version: 1,
  metadata: {
    component: { type: 'application', name: 'ThrottleWatch', version: '0.1.0' }
  },
  components
};
await writeFile(
  new URL('docs/licenses/sbom.cdx.json', root),
  `${JSON.stringify(document, null, 2)}\n`
);
process.stdout.write(`SBOM: ${components.length} dependencias\n`);
