// T112: leaves the collector ready to be bundled by `tauri build`.
//
// Publishes the sidecar self-contained (decision of 2026-09-26: the .NET runtime travels with
// the application, because installing the runtime needs administrator rights and T112 requires
// an installer that works per user without elevation) and then signs its release manifest, which
// `telemetry/launch.rs` verifies before starting anything.
//
// Must run BEFORE `tauri build`: the manifest has to exist inside the published folder for the
// bundler to pick it up as a resource.
//
//   node scripts/prepare-collector-bundle.mjs [--configuration Release] [--key <secret key>]
//                                             [--rsign <rsign.exe>] [--skip-signing]
//
// Signing uses the development key by default, which only a build with the `dev-signing` cargo
// feature trusts. A real release signs with the production key, which lives solely in the
// UPDATER_SIGNING_KEY GitHub secret (`.github/workflows/release.yml`).
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const option = (name) => {
  const index = args.indexOf(name);
  return index === -1 ? undefined : args[index + 1];
};
const fail = (message) => {
  console.error(`prepare-collector-bundle: ${message}`);
  process.exit(1);
};

const configuration = option('--configuration') ?? 'Release';
const project = path.join(repo, 'apps', 'sensor-agent', 'SensorAgent.csproj');
const publishDir = path.join(
  repo,
  'apps',
  'sensor-agent',
  'bin',
  configuration,
  'net10.0-windows',
  'win-x64',
  'publish'
);

const run = (command, commandArgs) => {
  const result = spawnSync(command, commandArgs, { stdio: 'inherit', shell: false });
  if (result.error) fail(`${command} could not be started: ${result.error.message}`);
  if (result.status !== 0) fail(`${command} exited with ${result.status}`);
};

console.log(`prepare-collector-bundle: publishing the sidecar self-contained (${configuration})`);
run('dotnet', [
  'publish',
  project,
  '-c',
  configuration,
  '-r',
  'win-x64',
  '--self-contained',
  'true',
  '-o',
  publishDir,
  '--nologo'
]);

const sidecar = path.join(publishDir, 'SensorAgent.exe');
if (!existsSync(sidecar)) fail(`the publish did not produce ${sidecar}`);

if (args.includes('--skip-signing')) {
  console.log('prepare-collector-bundle: signing skipped (--skip-signing)');
  process.exit(0);
}

console.log('prepare-collector-bundle: signing the release manifest');
// An explicit key means the release workflow handing over the production secret; without one
// this is a developer's machine signing with the development key.
const key = option('--key');
const signArgs = [
  path.join(repo, 'scripts', 'sign-release-manifest.mjs'),
  ...(key ? ['--production', '--key', key] : ['--dev']),
  '--dir',
  publishDir,
  '--version',
  '0.1.0-dev',
  '--file',
  'SensorAgent.exe',
  '--out',
  publishDir
];
const rsign = option('--rsign');
if (rsign) signArgs.push('--rsign', rsign);
run(process.execPath, signArgs);

console.log(`prepare-collector-bundle: ready at ${publishDir}`);
