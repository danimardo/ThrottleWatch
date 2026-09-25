import { expect, test } from './fixtures';

/**
 * E2E-13 (HU-19, FR-075): the two storage failures, forced on the real binary through the
 * `TW_DEV_*` fault injection of `src-tauri/src/dev_faults.rs` — not against the fake bridge, so
 * the whole path is exercised: Rust detects the failure, emits the event and the real window
 * reacts.
 *
 * Each scenario needs the app launched with its variable, so they run as two separate commands
 * (the harness launches one instance per run, in `global-setup.ts`):
 *
 *   # disco lleno: la escritura falla una vez y el backlog se recupera en el siguiente reintento
 *   $env:TW_DEV_STORAGE_FAIL_WRITES=1; pnpm exec playwright test --config=playwright.native.config.ts resilience
 *
 *   # base dañada: se aparta al arrancar y Ajustes ofrece exportarla
 *   $env:TW_DEV_CORRUPT_DB=1; pnpm exec playwright test --config=playwright.native.config.ts resilience
 *
 * A run without either variable skips both: there is nothing to observe in a healthy app.
 */

const WARNING = /can't save to disk|no se puede guardar en disco/i;

// `storage.retry_s` in ruleset-v1 is 60 s, so a recovery cannot be seen sooner than that.
const RETRY_BUDGET_MS = 90_000;

test('@critical disco lleno: aviso persistente, modo memoria y recuperacion sola', async ({
  page
}) => {
  test.skip(
    process.env.TW_DEV_STORAGE_FAIL_WRITES === undefined,
    'necesita TW_DEV_STORAGE_FAIL_WRITES; ver la cabecera de este fichero'
  );
  test.setTimeout(RETRY_BUDGET_MS + 60_000);

  // The warning is global: it must be there whatever screen the app happens to be on.
  await expect(page.getByText(WARNING)).toBeVisible({ timeout: 30_000 });

  // Nobody asks for the retry: Rust drains the backlog on its own and the warning goes away.
  await expect(page.getByText(WARNING)).toHaveCount(0, { timeout: RETRY_BUDGET_MS });
});

test('@critical base danada: se aparta al arrancar y Ajustes ofrece exportarla', async ({
  page
}) => {
  test.skip(
    process.env.TW_DEV_CORRUPT_DB === undefined,
    'necesita TW_DEV_CORRUPT_DB; ver la cabecera de este fichero'
  );

  // Recovering from corruption means a fresh database, so the app starts at onboarding.
  const skip = page.getByRole('button', { name: /omitir|skip/i });
  if (await skip.count()) {
    await skip.first().click();
  }

  await page.getByText('Ajustes', { exact: true }).first().click();
  const corruptRow = page.locator('.row', { hasText: /dañada|corrupt/i });
  await expect(corruptRow).toBeVisible({ timeout: 20_000 });

  // The offer names the file that was set aside, and offers to export it rather than delete it.
  await expect(corruptRow).toContainText(/throttlewatch\.db\.corrupt-/);
  await expect(corruptRow.getByRole('button')).toHaveCount(1);
});
