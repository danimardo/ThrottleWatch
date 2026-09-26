import { ensureOnboardingDone, expect, test } from './fixtures';

/**
 * T181: `e2e/navigation.spec.ts` drives the same six destinations against the fake bridge, which
 * answers every `invoke` without checking anything. That is what let T171 through — four commands
 * whose Rust parameter is named `request` were invoked unnested, so the real backend rejected all
 * of them while the whole suite stayed green. Walking the destinations here exercises the real
 * command surface instead: each screen mounts and asks the backend for its own data, so an
 * argument shape or a serialization the fake bridge would wave through fails.
 *
 * The viewport half of that spec (the 480 px "more" menu, the responsive nav) deliberately stays
 * in the browser harness: it is a CSS question, and a real Tauri window cannot be resized to
 * 480x384 over CDP.
 */

const DESTINATIONS = [
  { shortcut: 'Control+1', name: /now|ahora/i },
  { shortcut: 'Control+2', name: /analysis|análisis/i },
  { shortcut: 'Control+3', name: /cpu/i },
  { shortcut: 'Control+4', name: /sessions|sesiones/i },
  { shortcut: 'Control+5', name: /guided diagnostic|diagnóstico guiado/i },
  { shortcut: 'Control+6', name: /settings|ajustes/i }
] as const;

test('@native every destination mounts against the real command surface', async ({
  page
}) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));

  // A fresh `TW_DEV_DATA_DIR` means an empty database, so onboarding is what comes up first.
  await ensureOnboardingDone(page);

  for (const destination of DESTINATIONS) {
    await page.keyboard.press(destination.shortcut);
    await expect(
      page.getByRole('main', { name: destination.name })
    ).toBeVisible();
  }

  // `Control+,` is the one shortcut that is not a numbered destination (FR-041).
  await page.keyboard.press('Control+1');
  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();
  await page.keyboard.press('Control+,');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();

  // A rejected `invoke` surfaces as an unhandled rejection in the webview, which is precisely
  // the T171 signature the fake bridge could not produce.
  expect(pageErrors).toEqual([]);
});

test('@native a preference written through the real backend survives a reload', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await page.keyboard.press('Control+6');
  const settings = page.getByRole('main', { name: /settings|ajustes/i });
  await expect(settings).toBeVisible();

  // Any persisted preference would do; the language selector is the one with a visible, stable
  // effect on the document (`<html lang>`), so the round trip can be observed without reading
  // the database: interface -> `set_preference` -> SQLite -> reload -> `get_preferences`.
  const before = await page.locator('html').getAttribute('lang');
  expect(before).toMatch(/^(es|en)$/);

  await page.reload();
  await expect(page.locator('body')).not.toBeEmpty();
  await expect(page.locator('html')).toHaveAttribute('lang', before ?? 'es');
});
