import { ensureOnboardingDone, expect, test } from './fixtures';

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
  // This scenario needs live telemetry: the warning comes from a failed *write of a recorded
  // sample*, and with no collector there are none. The harness gives the app a test double for the
  // collector (`fake-collector.mjs`, via `TW_DEV_COLLECTOR_CMD`), so it runs the same on a CI runner
  // — which cannot sign the real collector's manifest — as on a developer machine. Before that it
  // could only run where a signed collector happened to be installed, and skipped itself under CI.
  test.setTimeout(RETRY_BUDGET_MS + 60_000);

  // T181: the run starts from an empty database now (`TW_DEV_DATA_DIR`), so onboarding is what
  // comes up first. Getting past it is what lets the application record.
  await ensureOnboardingDone(page);

  // The warning is global: it must be there whatever screen the app happens to be on.
  await expect(page.getByText(WARNING)).toBeVisible({ timeout: 30_000 });

  // Nobody asks for the retry: Rust drains the backlog on its own and the warning goes away. The
  // retry is paced by `storage.retry_s` (60 s), so with `TW_DEV_STORAGE_FAIL_WRITES=1` the first
  // retry succeeds and the warning clears within one period.
  //
  // Note the variable's value matters here: `=N` makes the first N writes fail, and each failed
  // retry costs another 60 s, so recovery takes about N minutes. Measured 2026-09-26 with `=3`:
  // failures at 0, 60 and 120 s. That is well past `RETRY_BUDGET_MS`, which is why this is
  // documented with `=1`.
  await expect(page.getByText(WARNING)).toHaveCount(0, {
    timeout: RETRY_BUDGET_MS
  });
});

test('@critical base danada: se aparta al arrancar y Ajustes ofrece exportarla', async ({
  page
}) => {
  test.skip(
    process.env.TW_DEV_CORRUPT_DB === undefined,
    'necesita TW_DEV_CORRUPT_DB; ver la cabecera de este fichero'
  );

  // Recovering from corruption means a fresh database, so the app starts at onboarding.
  await ensureOnboardingDone(page);

  // By shortcut and by a bilingual name, like the other native specs. This used to click the text
  // 'Ajustes', which only exists in a Spanish UI: on the CI runner (English Windows) it waited the
  // whole 30 s test timeout for a button called "Ajustes", and Playwright reported that as "Target
  // page, context or browser has been closed". Two earlier guesses at the cause (the CDP port, then
  // launching twice) were wrong; the failing log line said which text it was waiting for.
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();
  const corruptRow = page.locator('.row', { hasText: /dañada|corrupt/i });
  await expect(corruptRow).toBeVisible({ timeout: 20_000 });

  // The offer names the file that was set aside, and offers to export it rather than delete it.
  await expect(corruptRow).toContainText(/throttlewatch\.db\.corrupt-/);
  await expect(corruptRow.getByRole('button')).toHaveCount(1);
});
