import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
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
  const child = spawn(exePath, [], { stdio: 'ignore', windowsHide: true });
  const startupError = new Promise<never>((_resolve, reject) => {
    child.once('exit', (code) =>
      reject(new Error(`throttlewatch.exe exited early with code ${code}`))
    );
    child.once('error', reject);
  });

  await Promise.race([
    waitForCdp(Date.now() + READY_TIMEOUT_MS),
    startupError
  ]).catch((error) => {
    child.kill();
    throw error;
  });

  return async () => {
    child.kill();
  };
}
