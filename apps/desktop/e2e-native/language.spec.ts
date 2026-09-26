import {
  ensureOnboardingDone,
  expect,
  openScreen,
  tauriInvoke,
  test
} from './fixtures';

/**
 * T181 / language fix (2026-09-26). Two things the fake bridge could never show, because it decided
 * the language from the browser and never read what had been saved:
 *
 * - "Sistema" follows what *Rust* resolves from Windows' ranked language list, not what the WebView
 *   reports. The suite runs the application with the WebView's own language possibly different from
 *   Windows' (that is exactly what happened on the developer's machine), and the interface must
 *   agree with Rust regardless.
 * - Choosing a language applies at once. It used to be saved and never applied: the stored
 *   `locale.mode` was `"es"` and the screen stayed in English.
 */
const SETTINGS = /settings|ajustes/i;
const LANGUAGE_SELECT = /interface language|idioma de la interfaz/i;

async function effectiveLocale(page: import('@playwright/test').Page) {
  return tauriInvoke<'es' | 'en'>(page, 'get_effective_locale');
}

test('@native the interface follows the language Rust resolves, not the WebView', async ({
  page
}) => {
  await ensureOnboardingDone(page);

  const locale = await effectiveLocale(page);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);

  // The navigation is in that language: "Settings" or "Ajustes", never the other one.
  const settingsLabel = locale === 'es' ? /^ajustes$/i : /^settings$/i;
  await expect(
    page
      .getByRole('navigation')
      .first()
      .getByRole('button', { name: settingsLabel })
  ).toBeVisible();

  // ...and the WebView is free to disagree. Nothing here asserts on `navigator.language`, on purpose.

  // Settings says which language is in force and why, so the person is never left guessing: with
  // the choice on "Sistema" it names the language and that it comes from Windows.
  await openScreen(page, 'Control+6', SETTINGS);
  const effective = page.getByText(/idioma efectivo|effective language/i);
  await expect(effective).toContainText(
    locale === 'es' ? 'Español' : 'English'
  );
  await expect(effective).toContainText(/según windows|from windows/i);
});

test('@native choosing a language applies at once, keeps the screen and can be undone', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+6', SETTINGS);

  const original = await effectiveLocale(page);
  const other = original === 'es' ? 'en' : 'es';
  // The option is named in the language on screen: "Inglés"/"English", "Español"/"Spanish".
  const optionFor = (locale: 'es' | 'en') =>
    locale === 'es' ? /^espa[ñn]ol$|^spanish$/i : /^ingl[ée]s$|^english$/i;

  try {
    await page.getByRole('button', { name: LANGUAGE_SELECT }).click();
    await page.getByRole('option', { name: optionFor(other) }).click();

    // Applied without a reload...
    await expect(page.locator('html')).toHaveAttribute('lang', other);
    await expect(
      page
        .getByRole('navigation')
        .first()
        .getByRole('button', {
          name: other === 'es' ? /^ajustes$/i : /^settings$/i
        })
    ).toBeVisible();
    // ...Rust agrees, so its own texts (tray, notifications, the Now screen's labels) follow too...
    expect(await effectiveLocale(page)).toBe(other);
    // ...and the person is still on Settings, not dropped back on "Now"...
    await expect(page.getByRole('main', { name: SETTINGS })).toBeVisible();
    // ...where the effective-language line now says it is the person's own choice.
    const effective = page.getByText(/idioma efectivo|effective language/i);
    await expect(effective).toContainText(
      other === 'es' ? 'Español' : 'English'
    );
    await expect(effective).toContainText(/elegido por ti|your choice/i);
  } finally {
    // Leave the shared application as it was found: back to "Sistema".
    await tauriInvoke(page, 'set_preference', {
      request: {
        key: 'locale.mode',
        value: 'system',
        expected_schema_version: (
          await tauriInvoke<{ schema_version: number }>(page, 'get_preferences')
        ).schema_version
      }
    });
    await page.reload();
  }
  await ensureOnboardingDone(page);
  expect(await effectiveLocale(page)).toBe(original);
});
