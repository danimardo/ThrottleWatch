import {
  ensureOnboardingDone,
  expect,
  openScreen,
  tauriInvoke,
  test
} from './fixtures';

/**
 * T181: from `e2e/settings-lifecycle.spec.ts`, against the real application.
 *
 * Not migrated, deliberately: the compact tray layout is a viewport question (a real window cannot
 * be resized to 480x600 over CDP), and the corrupt-backup row is already covered end to end by
 * `resilience.spec.ts` with a genuinely corrupted database. Exporting a session needs a native
 * "save as" dialog, which CDP cannot reach.
 */
const SETTINGS = /settings|ajustes/i;

test('@native settings keeps deleting data and resetting the application apart', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+6', SETTINGS);

  await page.getByRole('button', { name: /delete data|borrar datos/i }).click();
  await expect(page.locator('dialog[open] h2')).toHaveText(
    /delete data|borrar datos/i
  );
  // Cancel, never confirm: this only checks that the confirmation is its own step.
  await page
    .locator('dialog[open]')
    .getByRole('button', { name: /cancel|cancelar/i })
    .click();
  await expect(page.locator('dialog[open]')).toHaveCount(0);

  await page
    .getByRole('button', {
      name: /reset throttlewatch|restablecer throttlewatch/i
    })
    .click();
  await expect(page.locator('dialog[open] h2')).toHaveText(
    /reset throttlewatch|restablecer throttlewatch/i
  );
  await page
    .locator('dialog[open]')
    .getByRole('button', { name: /cancel|cancelar/i })
    .click();
  await expect(page.locator('dialog[open]')).toHaveCount(0);
});

test('@native an active session does not expose the delete action', async ({
  page
}) => {
  // With the test-double collector the application records a passive session on its own, so the
  // "in progress" session the fake bridge had to be told about (`?active-session`) really exists.
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+4', /sessions|sesiones/i);

  const inProgress = page
    .getByRole('listitem')
    .filter({
      has: page
        .locator('.status-tag')
        .filter({ hasText: /in progress|en curso/i })
    });
  await expect(inProgress).toHaveCount(1, { timeout: 30_000 });

  // Scoped to *that* card. The original asserted no delete button anywhere on the screen, which
  // held only because its fake profile had a single session. Here the application is shared: an
  // earlier spec leaves finished guided sessions on the list, and those rightly offer deletion.
  await expect(
    inProgress.getByRole('button', { name: /delete session|eliminar sesión/i })
  ).toHaveCount(0);
});

test('@native the quiet period offers all 24 hours, not just 22 and 07 (T174)', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+6', SETTINGS);

  // Three sections fold under "Advanced" (monitoring, tray, about, in that order); the quiet
  // period lives in the tray section's, the second one on the page.
  await page
    .getByRole('button', { name: /^advanced$|^avanzado$/i })
    .nth(1)
    .click();

  const quietSwitch = page.getByRole('switch', {
    name: /quiet period|periodo de silencio/i
  });
  if (!(await quietSwitch.isChecked())) await quietSwitch.click();
  await expect(quietSwitch).toBeChecked();

  const startSelect = page.getByRole('button', { name: /^start$|^inicio$/i });
  await startSelect.click();
  const options = page.getByRole('option');
  await expect(options).toHaveCount(24);
  await expect(options.filter({ hasText: '00:00' })).toHaveCount(1);
  await expect(options.filter({ hasText: '23:00' })).toHaveCount(1);

  await options.filter({ hasText: '15:00' }).click();
  await expect(startSelect).toHaveText('15:00');

  // The fake bridge could only be asked which `set_preference` calls it had seen. Here the
  // preference is read back from the real backend: it was stored, not just sent.
  const snapshot = await tauriInvoke<{ values: Record<string, unknown> }>(
    page,
    'get_preferences'
  );
  const quiet = snapshot.values['notifications.quiet_period'] as
    { start?: string | number } | undefined;
  expect(String(quiet?.start)).toMatch(/^15/);
});
