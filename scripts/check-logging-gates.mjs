// Constitution XVII, «Cumplimiento»: a logging rule only counts as in force if the mechanism that
// enforces it exists *and* fails when the rule is broken. Until 2026-09-28 the constitution named a
// Clippy gate that `clippy.toml` did not contain, and 34 direct `tracing` calls passed Clippy clean.
//
//   node scripts/check-logging-gates.mjs          configuration + ESLint on violating samples
//   node scripts/check-logging-gates.mjs --full   also runs Clippy on a violating throwaway crate
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ESLint } from 'eslint';

const root = fileURLToPath(new URL('..', import.meta.url));
const crate = join(root, 'apps', 'desktop', 'src-tauri');
const failures = [];
const fail = (message) => failures.push(message);

// --- Rust: the configuration the constitution names ---------------------------------------------
const REQUIRED_DISALLOWED = [
  'std::print',
  'std::println',
  'std::dbg',
  'tracing::trace',
  'tracing::debug',
  'tracing::info',
  'tracing::warn',
  'tracing::error',
  'tracing::event'
];
const clippyToml = readFileSync(join(crate, 'clippy.toml'), 'utf8');
const disallowed = new Set([...clippyToml.matchAll(/path\s*=\s*"([^"]+)"/g)].map((match) => match[1]));
for (const path of REQUIRED_DISALLOWED) {
  if (!disallowed.has(path)) fail(`clippy.toml: disallowed-macros no incluye ${path}`);
}
// `eprintln!` is covered by `print_stderr`, not by `disallowed-macros`: `tauri::generate_context!`
// expands to one, and only the restriction lint skips third-party macro expansions.
const cargoToml = readFileSync(join(crate, 'Cargo.toml'), 'utf8');
const lints = cargoToml.split('[lints.clippy]')[1]?.split(/\n\[/)[0] ?? '';
for (const lint of ['print_stdout', 'print_stderr', 'dbg_macro', 'disallowed_macros']) {
  if (!new RegExp(`^${lint}\\s*=\\s*"deny"`, 'm').test(lints)) {
    fail(`Cargo.toml: [lints.clippy] no deniega ${lint}`);
  }
}

// Clippy is the real gate; this catches a violation even where Clippy was not run, and any attempt
// to silence the lint instead of going through the wrapper.
const DIRECT_TRACING = /\btracing::(trace|debug|info|warn|error|event)!\s*\(/;
const SILENCED = /allow\s*\(\s*clippy::disallowed_macros/;
const walk = (directory) =>
  readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? walk(path) : path.endsWith('.rs') ? [path] : [];
  });
for (const file of walk(join(crate, 'src'))) {
  const lines = readFileSync(file, 'utf8').split(/\r?\n/);
  lines.forEach((line, index) => {
    const where = `${relative(root, file)}:${index + 1}`;
    if (DIRECT_TRACING.test(line)) fail(`${where}: uso directo de tracing, usa log_*!`);
    if (SILENCED.test(line)) fail(`${where}: #[allow(clippy::disallowed_macros)] no está permitido`);
  });
}

// --- Interface: ESLint must reject console.* and loglevel in .ts and .svelte ----------------------
const eslint = new ESLint({ cwd: root });
for (const sample of ['apps/desktop/src/features/__gate__.ts', 'apps/desktop/src/features/__gate__.svelte']) {
  const config = await eslint.calculateConfigForFile(join(root, sample));
  const noConsole = config.rules?.['no-console'];
  const restricted = config.rules?.['no-restricted-imports'];
  const severity = (rule) => (Array.isArray(rule) ? rule[0] : rule);
  if (![2, 'error'].includes(severity(noConsole))) fail(`${sample}: no-console no está en error`);
  if (!JSON.stringify(restricted ?? '').includes('loglevel')) {
    fail(`${sample}: no-restricted-imports no bloquea loglevel`);
  }
}
const svelteSample = [
  '<script lang="ts">',
  "  import log from 'loglevel';",
  "  console.log('direct');",
  '  log.info("x");',
  '</script>'
].join('\n');
const [result] = await eslint.lintText(svelteSample, {
  filePath: join(root, 'apps/desktop/src/features/__gate__.svelte')
});
const ruleIds = new Set(result.messages.map((message) => message.ruleId));
for (const rule of ['no-console', 'no-restricted-imports']) {
  if (!ruleIds.has(rule)) {
    fail(`ESLint no rechaza la violación de ${rule} en un .svelte (mensajes: ${JSON.stringify(result.messages)})`);
  }
}

// --- --full: Clippy itself must fail on a violation under the repository's configuration ---------
if (process.argv.includes('--full')) {
  const fixture = mkdtempSync(join(tmpdir(), 'tw-logging-gate-'));
  try {
    mkdirSync(join(fixture, 'src'));
    writeFileSync(
      join(fixture, 'Cargo.toml'),
      '[package]\nname = "logging_gate_fixture"\nversion = "0.0.0"\nedition = "2024"\n\n' +
        '[dependencies]\ntracing = "=0.1.44"\n\n[lints.clippy]\ndisallowed_macros = "deny"\n'
    );
    writeFileSync(join(fixture, 'clippy.toml'), clippyToml);
    writeFileSync(join(fixture, 'src', 'lib.rs'), 'pub fn violate() {\n    tracing::warn!("direct");\n}\n');
    let rejected = false;
    try {
      execFileSync('cargo', ['clippy', '--offline', '--quiet', '--', '-D', 'warnings'], {
        cwd: fixture,
        stdio: 'pipe',
        env: { ...process.env, CARGO_TARGET_DIR: join(fixture, 'target') }
      });
    } catch (error) {
      rejected = String(error.stderr).includes('disallowed macro');
      if (!rejected) fail(`Clippy falló por otro motivo: ${String(error.stderr).slice(0, 400)}`);
    }
    if (!rejected && failures.length === 0) fail('Clippy aceptó tracing::warn! con la configuración del repositorio');
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
}

if (failures.length > 0) {
  console.error(`check-logging-gates: ${failures.length} fallo(s)\n- ${failures.join('\n- ')}`);
  process.exit(1);
}
console.log(`check-logging-gates: OK${process.argv.includes('--full') ? ' (incluido Clippy)' : ''}`);
