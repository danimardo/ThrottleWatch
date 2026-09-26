import { spawn } from 'node:child_process';
import { once } from 'node:events';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync
} from 'node:fs';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import path from 'node:path';

/**
 * T-PLAY-002 (`docs/spikes/e2e-webview2.md`): drives the real Tauri window instead of the Vite
 * preview server `frontend`/`app` use. The spike found `tauri dev` unsuitable for this (it needs
 * a machine-specific `beforeDevCommand` override) — this launches the built binary directly,
 * which embeds the production frontend build (`custom-protocol`) and needs no dev server at all.
 *
 * The suite has to hold in both interface languages: the CI runner is an English Windows, a Spanish
 * developer machine is not, and a test written against one silently fails on the other. To run it in
 * English locally: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--lang=en-US --accept-lang=en-US"`.
 *
 * Build first (not done here — this is a smoke you run deliberately, not part of `pnpm test:e2e`):
 *   pnpm exec vite build
 *   cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target-e2e
 */

const CDP_PORT = 9222;
const CDP_URL = `http://127.0.0.1:${CDP_PORT}/json/version`;
const READY_TIMEOUT_MS = 20_000;
const POLL_INTERVAL_MS = 300;

// A target directory of its own, not `target/debug`. `arrancar.ps1` launches whatever is in
// `target/debug`, and this suite needs a build with the `e2e` feature (it opens a CDP debugging
// port and reads the `TW_DEV_*` seams). Sharing the directory meant every suite build replaced the
// developer's own binary with an instrumented one — and every plain `cargo build`/`cargo test`
// replaced this one with a build that has no CDP port. Two consumers, two directories.
const exePath = fileURLToPath(
  new URL(
    '../src-tauri/target-e2e/debug/throttlewatch.exe',
    import.meta.url
  )
);
const fakeCollectorPath = fileURLToPath(
  new URL('./fake-collector.mjs', import.meta.url)
);
const seedHistoryPath = fileURLToPath(
  new URL('./seed-history.json', import.meta.url)
);

async function waitForCdp(deadline: number): Promise<void> {
  while (Date.now() < deadline) {
    try {
      const response = await fetch(CDP_URL);
      if (response.ok) return;
    } catch {
      // Not listening yet.
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
  throw new Error(
    `throttlewatch.exe did not open the CDP port at ${CDP_URL} within ${READY_TIMEOUT_MS}ms. ` +
      'The port is only opened by a build with the `e2e` feature (target-e2e/, kept apart from ' +
      "target/debug so it is never overwritten by a plain build): rebuild it with `cargo build --locked " +
      '--features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target-e2e`.'
  );
}

/** Whether something is already answering on the CDP port. */
async function cdpAnswers(): Promise<boolean> {
  try {
    return (await fetch(CDP_URL)).ok;
  } catch {
    return false;
  }
}

export default async function globalSetup(): Promise<() => Promise<void>> {
  // The application is single-instance: a second launch forwards to the running one and exits, and
  // a stale `throttlewatch.exe` from an earlier run still owns this CDP port. Without this check the
  // harness attaches to that old process — with the *old* environment (no test-double collector, a
  // different data directory) — and every result is about something other than the run just asked
  // for. It cost an afternoon's worth of misleading failures (2026-09-26): the guided scenarios
  // reported "no sensors" for code that was fine.
  if (await cdpAnswers()) {
    throw new Error(
      `something already answers on ${CDP_URL}: most likely a throttlewatch.exe left over from an earlier run. ` +
        'Close it (Get-Process throttlewatch) and run again; attaching to it would test the wrong process.'
    );
  }
  if (!existsSync(exePath)) {
    throw new Error(
      `${exePath} does not exist. Build it first: cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target-e2e (and pnpm exec vite build for the embedded frontend).`
    );
  }
  // T181: this drives the real binary, which without help would use the real
  // `%APPDATA%\com.throttlewatch.desktop` — the developer's own database, history and
  // preferences. That would make every run depend on whatever state that machine happens to
  // carry (green locally, red in CI) and put real data at risk. `TW_DEV_DATA_DIR` moves the
  // whole directory to a throwaway one, so each run starts from an empty database.
  //
  // Redirecting `APPDATA` instead does not work: Tauri resolves the directory through
  // `SHGetKnownFolderPath`, which ignores that variable (measured 2026-09-26 — the target
  // folder stayed empty). Hence the explicit seam, compiled out of release builds.
  const dataDir = mkdtempSync(path.join(os.tmpdir(), 'throttlewatch-e2e-'));

  // E2E-13: the `TW_DEV_*` fault variables reach the app from whoever runs the suite, so the
  // same harness covers the healthy run and the two injected failures without a second config.
  // They are read only by debug/`e2e` builds (`src-tauri/src/dev_faults.rs`). An explicit
  // `TW_DEV_DATA_DIR` from the caller wins, so a failing run can be pointed at a kept directory.
  const env = {
    ...process.env,
    TW_DEV_DATA_DIR: process.env.TW_DEV_DATA_DIR ?? dataDir,
    // One finished session with samples and a report, imported at startup through the product's own
    // importer (`TW_DEV_SEED_BUNDLE`), so scenarios about sessions have something to open, export and
    // reference without minutes of recording or a native dialog. A caller's value wins.
    TW_DEV_SEED_BUNDLE: process.env.TW_DEV_SEED_BUNDLE ?? seedHistoryPath,
    ...collectorEnvironment()
  };

  // `TW_DEV_CORRUPT_DB` damages an existing database; it never creates one, on purpose (see
  // `dev_faults::corrupt_database_if_requested`). With the isolated directory above the run starts
  // empty, so there would be nothing to damage and the scenario would silently test nothing.
  //
  // It only needs a *file* to damage, not a database the app wrote, so one is placed here. This used
  // to launch the app once to seed a real one, kill it and launch again; that was simplified away
  // on the suspicion that it caused the CI failure of this scenario, but it did not (that failure
  // was the test clicking the Spanish-only text 'Ajustes' on an English runner — see resilience.spec.ts).
  // It is kept because a placeholder file is simpler than starting the app twice on one WebView2
  // profile, not because launching twice was shown to be a problem.
  if (env.TW_DEV_CORRUPT_DB !== undefined) {
    mkdirSync(env.TW_DEV_DATA_DIR, { recursive: true });
    writeFileSync(
      path.join(env.TW_DEV_DATA_DIR, 'throttlewatch.db'),
      placeholderDatabase()
    );
  }

  const child = await launch(env).catch(async (error: unknown) => {
    await removeWhenReleased(dataDir);
    throw error;
  });

  return async () => {
    expectExit(child);
    child.kill();
    // `kill()` returns before Windows has released the SQLite files, so removing the directory
    // straight away fails with EPERM. Wait for the process to actually be gone first.
    await once(child, 'exit').catch(() => undefined);
    // Only the directory this setup created is removed. A `TW_DEV_DATA_DIR` the caller supplied
    // is left alone: they asked for it on purpose, most likely to inspect it after a failure.
    if (!process.env.TW_DEV_DATA_DIR) {
      await removeWhenReleased(dataDir);
    }
  };
}

/**
 * Which collector the run under test gets.
 *
 * By default the test double (`fake-collector.mjs`). The real collector only starts from a manifest
 * signed by a trusted key, and the private half of the development key is kept out of the repository
 * on purpose — so on a CI runner (or any machine that has not signed one) the app ran with no
 * telemetry at all, and every scenario that needs samples was unreachable. It also made the suite
 * depend on whether the developer had re-signed after their last `dotnet build`. The double is the
 * same on every machine, which is the point of an E2E fixture; the real collector has its own tests
 * (.NET suite, T-INT).
 *
 * `TW_E2E_REAL_COLLECTOR=1` opts out, for a run that wants the real one on a machine that has it. A
 * `TW_DEV_COLLECTOR_CMD` from the caller always wins.
 */
function collectorEnvironment(): NodeJS.ProcessEnv {
  if (
    process.env.TW_DEV_COLLECTOR_CMD !== undefined ||
    process.env.TW_E2E_REAL_COLLECTOR === '1'
  ) {
    return {};
  }
  return {
    TW_DEV_COLLECTOR_CMD: JSON.stringify([process.execPath, fakeCollectorPath])
  };
}

/**
 * A file the size of a small database that starts with SQLite's magic header. It is not a database
 * the app ever wrote and does not need to be: `TW_DEV_CORRUPT_DB` overwrites that header, which is
 * what makes SQLite reject it as "not a database" — the condition the app's recovery looks for.
 */
function placeholderDatabase(): Buffer {
  const file = Buffer.alloc(8192);
  file.write('SQLite format 3\0', 0, 'latin1');
  return file;
}

const expectedExits = new WeakSet<object>();

/** Marks a child this setup is about to kill itself, so its exit is not reported as a crash. */
function expectExit(child: object): void {
  expectedExits.add(child);
}

/** Starts the binary and resolves once its CDP port answers, or rejects if it exits first. */
async function launch(
  env: NodeJS.ProcessEnv
): Promise<ReturnType<typeof spawn>> {
  const child = spawn(exePath, [], {
    stdio: 'ignore',
    windowsHide: true,
    env
  });
  const startupError = new Promise<never>((_resolve, reject) => {
    child.once('exit', (code) =>
      reject(new Error(`throttlewatch.exe exited early with code ${code}`))
    );
    child.once('error', reject);
  });

  try {
    await Promise.race([
      waitForCdp(Date.now() + READY_TIMEOUT_MS),
      startupError
    ]);
  } catch (error) {
    child.kill();
    throw error;
  }
  // The rejection above is no longer anyone's to handle once the window is up, and an unhandled
  // one would crash the runner when the app is later killed in teardown.
  startupError.catch(() => undefined);
  // Once the window is up, an exit nobody asked for is the app dying under the test. Playwright then
  // only reports "Target page, context or browser has been closed", which says nothing about why
  // (seen on the CI runner for the corrupt-database scenario, never locally) — so say it here.
  child.once('exit', (code, signal) => {
    if (!expectedExits.has(child)) {
      console.error(
        `[e2e-native] throttlewatch.exe (pid ${child.pid}) exited on its own: code=${code} signal=${signal}`
      );
    }
  });
  return child;
}

/**
 * Best effort on purpose: a leftover directory under the OS temp folder is harmless, while a
 * throwing teardown turns a green suite red for a reason that has nothing to do with the app.
 */
async function removeWhenReleased(directory: string): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      rmSync(directory, { recursive: true, force: true });
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }
}
