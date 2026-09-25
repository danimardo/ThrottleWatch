// DEVELOPMENT-ONLY: builds and signs the release manifest that the elevated launcher verifies
// (ADR-0004 R2/C3, apps/desktop/src-tauri/src/release_manifest.rs).
//
//   node scripts/sign-release-manifest.mjs --dev --dir <install dir> --version <v> \
//        --file SensorAgent.exe [--file other.dll ...] [--out <dir>] [--key <secret key>] [--rsign <rsign.exe>]
//
// Writes release-manifest.json and release-manifest.json.minisig into --out (default: --dir).
// The signature is made with the DEVELOPMENT key (default: %LOCALAPPDATA%\ThrottleWatch-signing\
// dev-release.key, public half in apps/desktop/src-tauri/keys/dev-release.pub). Release manifests are
// signed in protected CI with the updater key (Tauri signer), never with this script or this key.
// Requires the minisign-compatible tool: `cargo install rsign2 --locked` (tested with 0.6.6).
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);

function fail(message) {
  console.error(`sign-release-manifest: ${message}`);
  process.exit(1);
}

function option(name, { multiple = false } = {}) {
  const values = [];
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === name) {
      const value = args[index + 1];
      if (value === undefined || value.startsWith('--')) {
        fail(`${name} needs a value`);
      }
      values.push(value);
    }
  }
  return multiple ? values : values.at(-1);
}

if (!args.includes('--dev')) {
  fail(
    'this script signs with the DEVELOPMENT key only; pass --dev to confirm'
  );
}
const dir = option('--dir');
const version = option('--version');
const files = option('--file', { multiple: true });
if (!dir || !version || files.length === 0) {
  fail(
    'usage: --dev --dir <dir> --version <v> --file <name> [--file <name> ...] [--out <dir>] [--key <path>] [--rsign <path>]'
  );
}
const out = option('--out') ?? dir;
// Deliberately NOT under %LOCALAPPDATA%\ThrottleWatch: that is where the NSIS installer puts the
// application (`installMode: currentUser` resolves to `$LOCALAPPDATA\ThrottleWatch`), so private
// signing keys used to sit inside the install directory — one template change away from being
// removed by an uninstall (T112, 2026-09-25).
const key =
  option('--key') ??
  path.join(
    process.env.LOCALAPPDATA ?? '',
    'ThrottleWatch-signing',
    'dev-release.key'
  );
const rsign = option('--rsign') ?? 'rsign';
if (!existsSync(key)) {
  fail(`development key not found: ${key}`);
}

// Same rules the Rust side enforces: plain relative paths only.
function checkRelative(value) {
  if (
    value.length === 0 ||
    value.includes(':') ||
    path.isAbsolute(value) ||
    value
      .split(/[\\/]/)
      .some((part) => part === '..' || part === '.' || part === '')
  ) {
    fail(`"${value}" is not a plain relative path`);
  }
}

const entries = files.map((relative) => {
  checkRelative(relative);
  const file = path.join(dir, relative);
  if (!existsSync(file)) {
    fail(`file not found: ${file}`);
  }
  return {
    path: relative,
    sha256: createHash('sha256').update(readFileSync(file)).digest('hex')
  };
});

mkdirSync(out, { recursive: true });
const manifestPath = path.join(out, 'release-manifest.json');
const signaturePath = `${manifestPath}.minisig`;
const manifest = { manifest_version: 1, version, files: entries };
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

try {
  execFileSync(
    rsign,
    [
      'sign',
      '-W',
      '-s',
      key,
      '-x',
      signaturePath,
      '-t',
      `ThrottleWatch release manifest ${version}`,
      '-c',
      'signed with the ThrottleWatch development key',
      manifestPath
    ],
    { stdio: ['ignore', 'pipe', 'pipe'] }
  );
} catch (error) {
  fail(
    `rsign failed (${error instanceof Error ? error.message : error}); install it with "cargo install rsign2 --locked"`
  );
}
console.log(
  `sign-release-manifest: wrote ${manifestPath} and ${signaturePath} (${entries.length} file(s), DEV key)`
);
