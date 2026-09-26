import { ensureOnboardingDone, expect, openScreen, test } from './fixtures';

/**
 * T181: from `e2e/sessions-export.spec.ts` (first test) and `e2e/shortcuts.spec.ts` (Ctrl+E),
 * against the real application, with a session that really is in its database: the harness starts
 * it with `TW_DEV_SEED_BUNDLE`, which imports `seed-history.json` through the product's own importer.
 *
 * Deliberately not here:
 * - "Use as reference": only a *completed guided* session can be one, and an import is always an
 *   imported session, so the seed cannot produce it. Left to the fake bridge.
 * - Saving and importing a file: both go through a native "save as"/"open" dialog, which CDP cannot
 *   reach, and a variable to bypass it is not worth it for one scenario.
 * - F1: it opens the help file with the operating system's opener — a side effect on the machine
 *   running the suite, and one nobody can observe from the page.
 *
 * Nothing here confirms a deletion: the application is shared, and the seeded session must still be
 * there for whoever runs next.
 */
const SESSIONS = /sessions|sesiones/i;

/** The seeded session's card: the only one tagged "Imported". */
function importedCard(page: import('@playwright/test').Page) {
  return page.getByRole('listitem').filter({
    has: page.locator('.status-tag.imported')
  });
}

test('@native the seeded session is on the Sessions screen', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+4', SESSIONS);

  // If the seed did not land, every other scenario in this file fails for a reason that has
  // nothing to do with what it tests; this one says so plainly.
  const card = importedCard(page);
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await expect(card.getByRole('button', { name: /open|abrir/i })).toBeVisible();
});

test('@native a session opens its report and returns to the list', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+4', SESSIONS);

  await importedCard(page)
    .getByRole('button', { name: /open|abrir/i })
    .click();
  const back = page.getByRole('button', {
    name: /back to sessions|volver a sesiones/i
  });
  await expect(back).toBeVisible();

  await back.click();
  await expect(importedCard(page)).toHaveCount(1);
});

test('@native deleting a session asks for confirmation first', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+4', SESSIONS);

  await importedCard(page)
    .getByRole('button', { name: /delete session|eliminar sesión/i })
    .click();
  const dialog = page.locator('dialog[open]');
  await expect(dialog).toBeVisible();

  // Cancel, never confirm: it is the confirmation that is under test, and the session has to
  // survive for the next spec.
  await dialog.getByRole('button', { name: /cancel|cancelar/i }).click();
  await expect(dialog).toHaveCount(0);
  await expect(importedCard(page)).toHaveCount(1);
});

test('@native Ctrl+E opens the export dialog on the Sessions screen', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await page.keyboard.press('Control+e');

  await expect(page.getByRole('main', { name: SESSIONS })).toBeVisible();
  // It is the export dialog, not just *a* dialog: the original only asserted the latter, which
  // would have been satisfied by any modal left open.
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('heading').first()).toHaveText(
    /export|exportar/i
  );
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
});
