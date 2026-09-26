import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import fs from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import path from 'node:path';

/**
 * T-PLAY-002 (`docs/spikes/e2e-webview2.md`): drives the real Tauri window instead of the Vite
 * preview server `frontend`/`app` use. The spike found `tauri dev` unsuitable for this (it needs
 * a machine-specific `beforeDevCommand` override) — this launches the built binary directly,
 * which embeds the production frontend build (`custom-protocol`) and needs no dev server at all.
 *
 * Build first (not done here — this is a smoke you run deliberately, not part of `pnpm test:e2e`):
 *   pnpm exec vite build
 *   cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml
 */

const CDP_PORT = 9222;
const CDP_URL = `http://127.0.0.1:${CDP_PORT}/json/version`;
const READY_TIMEOUT_MS = 20_000;
const POLL_INTERVAL_MS = 300;

const exePath = fileURLToPath(
  new URL('../src-tauri/target/debug/throttlewatch.exe', import.meta.url)
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
    `throttlewatch.exe did not open the CDP port at ${CDP_URL} within ${READY_TIMEOUT_MS}ms`
  );
}

export default async function globalSetup(): Promise<() => Promise<void>> {
  if (!existsSync(exePath)) {
    throw new Error(
      `${exePath} does not exist. Build it first: cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml (and pnpm exec vite build for the embedded frontend).`
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
    TW_DEV_DATA_DIR: process.env.TW_DEV_DATA_DIR ?? dataDir
  };

  // `TW_DEV_CORRUPT_DB` damages an existing database; it never creates one, on purpose (see
  // `dev_faults::corrupt_database_if_requested`). With the isolated directory above the run
  // starts empty, so there would be nothing to damage and the scenario would silently test
  // nothing. One throwaway launch seeds a real database first, then the run under test opens it
  // and has to recover. Before the directory was isolated this scenario quietly depended on the
  // developer's own database already being there — on a clean machine it proved nothing.
  if (env.TW_DEV_CORRUPT_DB !== undefined) {
    const { TW_DEV_CORRUPT_DB: _corrupt, ...healthy } = env;
    const database = path.join(env.TW_DEV_DATA_DIR, 'throttlewatch.db');
    const seed = await launch(healthy);
    // The CDP port answers as soon as the window exists, and `lib.rs` builds the window *before*
    // it resolves the data directory and opens the database. Killing on CDP alone therefore
    // usually killed the seed before the file existed (measured 2026-09-26).
    await waitUntilExists(database);
    seed.kill();
    await once(seed, 'exit').catch(() => undefined);
    // The seed's WebView2 child processes can outlive it and keep answering on the CDP port for a
    // moment. Launching straight away made `waitForCdp` see that dying browser as the new app, and
    // Playwright then attached to it ("Target page, context or browser has been closed", seen on
    // the CI runner, never locally). Wait until nothing answers before starting the run under test.
    await waitUntilCdpClosed();

    // Windows keeps the SQLite files locked for a moment after the process is gone. The run under
    // test opens the database immediately, and `overwrite_header` reports failure by returning
    // false without logging, so losing this race would leave the database intact and the scenario
    // passing vacuously. Wait for the file to be writable, and refuse to continue if it never is.
    if (!(await waitUntilWritable(database))) {
      throw new Error(
        `the seeding launch did not leave a writable database at ${database}, so TW_DEV_CORRUPT_DB would have had nothing to damage`
      );
    }
  }

  const child = await launch(env).catch(async (error: unknown) => {
    await removeWhenReleased(dataDir);
    throw error;
  });

  return async () => {
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

/** Waits until nothing answers on the CDP port any more. */
async function waitUntilCdpClosed(): Promise<void> {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  while (Date.now() < deadline) {
    try {
      await fetch(CDP_URL);
    } catch {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
  throw new Error(
    `something still answers on ${CDP_URL} ${READY_TIMEOUT_MS}ms after the seeding launch was killed`
  );
}

/** Waits for `file` to show up, for as long as the CDP handshake is given. */
async function waitUntilExists(file: string): Promise<void> {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  while (Date.now() < deadline && !existsSync(file)) {
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}

/** Whether `file` exists and can be opened for writing, retried while Windows releases it. */
async function waitUntilWritable(file: string): Promise<boolean> {
  for (let attempt = 0; attempt < 25; attempt += 1) {
    try {
      const handle = await fs.open(file, 'r+');
      await handle.close();
      return true;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }
  return false;
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
