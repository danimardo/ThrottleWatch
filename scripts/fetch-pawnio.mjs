// Fetches the official PawnIO installer at BUILD time so the repository does not host the binary.
//
//   node scripts/fetch-pawnio.mjs            download (if missing or different), verify, place
//   node scripts/fetch-pawnio.mjs --check    only verify the file already in place; exit 1 otherwise
//   node scripts/fetch-pawnio.mjs --out DIR  place the installer under DIR instead of src-tauri/resources
//
// Source of truth: apps/desktop/src-tauri/resources/pawnio-manifest.json (version, SHA-256,
// expected Authenticode publisher). Nothing is written unless BOTH checks pass:
//   1. SHA-256 of the downloaded bytes equals the manifest.
//   2. Get-AuthenticodeSignature reports Valid and the signer equals `publisher_subject`.
// The application never downloads PawnIO at run time (FR-087, ADR-0004 R5, constitution:
// the only network traffic is the updater); this runs on the developer's machine or in CI,
// and the verified file is then bundled into the NSIS installer as a resource (T153).
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const defaultResources = path.join(
  root,
  'apps',
  'desktop',
  'src-tauri',
  'resources'
);
const args = process.argv.slice(2);
const checkOnly = args.includes('--check');
const outIndex = args.indexOf('--out');
const resourcesDir =
  outIndex >= 0 ? path.resolve(args[outIndex + 1] ?? '') : defaultResources;

const manifest = JSON.parse(
  readFileSync(path.join(defaultResources, 'pawnio-manifest.json'), 'utf8')
);
for (const key of [
  'packaged_version',
  'installer',
  'sha256',
  'publisher_subject'
]) {
  if (typeof manifest[key] !== 'string' || manifest[key].length === 0) {
    fail(`pawnio-manifest.json: missing "${key}"`);
  }
}
const url = `https://github.com/namazso/PawnIO.Setup/releases/download/${manifest.packaged_version}/PawnIO_setup.exe`;
const target = path.join(resourcesDir, manifest.installer);

function fail(message) {
  throw new Error(message);
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

// Drop the PowerShell 7 module path inherited from the CI shell so 5.1 uses its own defaults.
function windowsPowerShellEnvironment() {
  const environment = { ...process.env };
  delete environment.PSModulePath;
  return environment;
}

function verifySignature(file) {
  if (process.platform !== 'win32') {
    fail('the Authenticode signature can only be verified on Windows');
  }
  // `powershell -Command` does not forward extra arguments as $args: embed the escaped path.
  const literal = file.replaceAll("'", "''");
  // Explicit import from $PSHOME: on the CI runner this script is launched from PowerShell 7 and
  // the inherited PSModulePath makes Windows PowerShell 5.1 fail to autoload the module.
  const script =
    "Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security') -ErrorAction Stop; " +
    `$s = Get-AuthenticodeSignature -LiteralPath '${literal}'; ` +
    '[pscustomobject]@{ status = "$($s.Status)"; subject = $s.SignerCertificate.Subject } | ConvertTo-Json -Compress';
  const raw = execFileSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', script],
    { encoding: 'utf8', env: windowsPowerShellEnvironment() }
  );
  const result = JSON.parse(raw);
  if (result.status !== 'Valid') {
    fail(`Authenticode status is "${result.status}", expected "Valid"`);
  }
  if (result.subject !== manifest.publisher_subject) {
    fail(
      `signer "${result.subject}" differs from the manifest publisher "${manifest.publisher_subject}"`
    );
  }
}

async function main() {
  if (existsSync(target) && sha256(readFileSync(target)) === manifest.sha256) {
    verifySignature(target);
    console.log(
      `fetch-pawnio: ok, ${path.relative(root, target)} matches the manifest (${manifest.sha256.slice(0, 12)}…)`
    );
    process.exit(0);
  }
  if (checkOnly) {
    fail(
      `${path.relative(root, target)} is missing or does not match the manifest; run "node scripts/fetch-pawnio.mjs"`
    );
  }

  console.log(`fetch-pawnio: downloading ${url}`);
  const response = await fetch(url, {
    redirect: 'follow',
    signal: AbortSignal.timeout(120_000)
  });
  if (!response.ok) {
    fail(`download failed: HTTP ${response.status}`);
  }
  const bytes = Buffer.from(await response.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== manifest.sha256) {
    fail(
      `SHA-256 mismatch: got ${actual}, manifest says ${manifest.sha256}; nothing written`
    );
  }

  mkdirSync(path.dirname(target), { recursive: true });
  const temporary = `${target}.tmp.exe`;
  writeFileSync(temporary, bytes);
  try {
    verifySignature(temporary);
  } catch (error) {
    rmSync(temporary, { force: true });
    throw error;
  }
  renameSync(temporary, target);
  console.log(
    `fetch-pawnio: wrote ${path.relative(root, target)} (${bytes.length} bytes, hash and signature verified)`
  );
}

try {
  await main();
} catch (error) {
  console.error(
    `fetch-pawnio: ${error instanceof Error ? error.message : error}`
  );
  process.exit(1);
}
