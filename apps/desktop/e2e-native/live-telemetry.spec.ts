import { ensureOnboardingDone, expect, test } from './fixtures';

/**
 * T181: the test double for the collector (`fake-collector.mjs`) is only worth having if what it
 * says really travels the whole path. The `capabilities` it sends name a CPU; that name reaches the
 * `Ahora` screen only if the app launched the double, completed the real IPC handshake with it,
 * accepted its catalog and published it — collector, Rust host, Tauri event and interface. With no
 * collector at all (what a CI runner had before) the screen says "CPU not detected yet" instead.
 *
 * It is also the guard that would notice the suite silently going back to running with no
 * telemetry, which is how the full-disk scenario failed in CI without anyone seeing why.
 */
test('@native the CPU the collector reports reaches the Now screen', async ({
  page
}) => {
  test.skip(
    process.env.TW_E2E_REAL_COLLECTOR === '1',
    'este test espera el colector falso; con el real la CPU es la de la máquina'
  );

  await ensureOnboardingDone(page);
  await page.keyboard.press('Control+1');
  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();

  await expect(page.getByText(/Test double CPU/)).toBeVisible({
    timeout: 30_000
  });
});
