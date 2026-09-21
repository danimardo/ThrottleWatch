import { readFile, writeFile } from 'node:fs/promises';
import { basename } from 'node:path';
import { parseArgs } from 'node:util';

/**
 * Builds `update-manifest.json` in the format `tauri-plugin-updater` reads, from an installer
 * and its minisign signature. It never sees a private key: signing happens in a separate step of
 * the release workflow (ADR-0005, T102).
 */
export function buildManifest({ version, notes, installerName, signatureText, baseUrl, publishedAt }) {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)) {
    throw new Error(`versión no válida: ${version}`);
  }
  if (!baseUrl.startsWith('https://')) {
    throw new Error('la URL base de las descargas debe ser https');
  }
  if (!signatureText.startsWith('untrusted comment:')) {
    throw new Error('la firma no tiene el formato minisign');
  }
  if (!/^[\w.-]+$/u.test(installerName)) {
    throw new Error(`nombre de instalador no válido: ${installerName}`);
  }
  return {
    version,
    notes,
    pub_date: publishedAt,
    platforms: {
      'windows-x86_64': {
        signature: Buffer.from(signatureText, 'utf8').toString('base64'),
        url: `${baseUrl.replace(/\/$/u, '')}/${installerName}`
      }
    }
  };
}

if (import.meta.filename === process.argv[1]) {
  const { values } = parseArgs({
    options: {
      version: { type: 'string' },
      'notes-file': { type: 'string' },
      installer: { type: 'string' },
      signature: { type: 'string' },
      'base-url': { type: 'string' },
      out: { type: 'string' }
    }
  });
  for (const required of ['version', 'notes-file', 'installer', 'signature', 'base-url', 'out']) {
    if (!values[required]) throw new Error(`falta --${required}`);
  }
  const manifest = buildManifest({
    version: values.version,
    notes: (await readFile(values['notes-file'], 'utf8')).trim(),
    installerName: basename(values.installer),
    signatureText: await readFile(values.signature, 'utf8'),
    baseUrl: values['base-url'],
    publishedAt: new Date().toISOString()
  });
  await writeFile(values.out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`update manifest: ${values.out} (${manifest.version})`);
}
