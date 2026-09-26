import { ensureOnboardingDone, expect, openScreen, test } from './fixtures';

/**
 * T181: from `e2e/updates.spec.ts`, only the "off by default" scenario.
 *
 * The other two (check/download/verify as separate gestures, and an operation in progress blocking
 * the install) need an update to exist: the real application asks GitHub for it and trusts only the
 * production key, and installing restarts it. There is no update server or signed artefact to point
 * it at, so those stay on the fake bridge.
 *
 * What this cannot show is the second half of the original assertion — that no `check_for_update`
 * was issued. That was read off the fake bridge's call log; the real updater runs in Rust and its
 * network traffic is not observable from the page. The disabled state is.
 */
test('@native updates are off by default: they cannot be checked', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+6', /settings|ajustes/i);

  await expect(
    page.getByRole('button', { name: /check now|buscar ahora/i })
  ).toBeDisabled();
  await expect(
    page.getByText(
      /turn on automatic update checks|activa la búsqueda automática/i
    )
  ).toBeVisible();
});
