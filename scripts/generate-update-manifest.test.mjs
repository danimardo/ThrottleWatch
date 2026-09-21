import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildManifest } from './generate-update-manifest.mjs';

const signatureText =
  'untrusted comment: signature from rsign secret key\nRUTi...\ntrusted comment: t\nabc==\n';
const base = {
  version: '1.2.0',
  notes: 'Correcciones',
  installerName: 'ThrottleWatch_1.2.0_x64-setup.exe',
  signatureText,
  baseUrl: 'https://github.com/ThrottleWatch/ThrottleWatch/releases/download/v1.2.0/',
  publishedAt: '2026-09-21T08:00:00.000Z'
};

test('the manifest has the shape the updater plugin reads', () => {
  const manifest = buildManifest(base);
  assert.equal(manifest.version, '1.2.0');
  const target = manifest.platforms['windows-x86_64'];
  assert.equal(
    target.url,
    'https://github.com/ThrottleWatch/ThrottleWatch/releases/download/v1.2.0/ThrottleWatch_1.2.0_x64-setup.exe'
  );
  assert.equal(Buffer.from(target.signature, 'base64').toString('utf8'), signatureText);
});

test('an invalid version, an insecure URL or a file that is not a minisign signature is refused', () => {
  assert.throws(() => buildManifest({ ...base, version: 'latest' }), /versión/u);
  assert.throws(() => buildManifest({ ...base, baseUrl: 'http://example.com' }), /https/u);
  assert.throws(() => buildManifest({ ...base, signatureText: 'nope' }), /minisign/u);
  assert.throws(() => buildManifest({ ...base, installerName: '../evil.exe' }), /instalador/u);
});
