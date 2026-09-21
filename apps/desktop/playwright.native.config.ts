import { defineConfig } from '@playwright/test';

/**
 * T-PLAY-002 (`docs/spikes/e2e-webview2.md`): drives the real Tauri `e2e` binary over CDP,
 * instead of the Vite preview server `playwright.config.ts`'s `frontend`/`app` projects use.
 * Deliberately separate from `playwright.config.ts`: this needs the binary built first (see
 * `e2e-native/global-setup.ts`) and is not part of `pnpm test:e2e` or CI yet — run it explicitly:
 *
 *   pnpm exec vite build
 *   cargo build --locked --features e2e,custom-protocol --manifest-path src-tauri/Cargo.toml
 *   pnpm exec playwright test --config playwright.native.config.ts
 */
export default defineConfig({
  testDir: './e2e-native',
  globalSetup: './e2e-native/global-setup.ts',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  projects: [{ name: 'native' }]
});
