import { ensureOnboardingDone, expect, openScreen, test } from './fixtures';

/**
 * A screen whose track names are raw sensor identifiers is untranslated in both languages at once,
 * so no catalog comparison ever finds it: it printed `temperature`, `clock`, `load` and `power`.
 */
test('@native the analysis tracks are named in the interface language, not by sensor id', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+2', /analysis|análisis/i);

  // Give the chart time to draw from the seeded session.
  await expect(
    page.getByRole('main', { name: /analysis|análisis/i })
  ).toBeVisible();
  await page.waitForTimeout(1500);

  // The legend names each track by a button, and its accessible name is what a screen reader says.
  // (A text search for the raw ids proved useless — it passed even against the bug — and so did a
  // negative check; the accessible name is the reliable handle.) Whichever language is on screen.
  const legend = page.getByRole('main', { name: /analysis|análisis/i });
  const tracks: Array<[RegExp, RegExp]> = [
    [/^temperature$/, /^(temperature|temperatura)$/i],
    [/^clock$/, /^(active clock|frecuencia activa)$/i],
    [/^load$/, /^(load|carga)$/i],
    [/^power$/, /^(package power|potencia de paquete)$/i]
  ];
  for (const [raw, named] of tracks) {
    // The raw sensor id must not be a button's name anywhere...
    await expect(legend.getByRole('button', { name: raw })).toHaveCount(0);
  }
  // ...and at least the tracks the seeded session has data for are named properly.
  await expect(
    legend.getByRole('button', { name: tracks[0]![1] })
  ).toBeVisible();
  await expect(
    legend.getByRole('button', { name: tracks[2]![1] })
  ).toBeVisible();
  await expect(
    legend.getByRole('button', { name: tracks[3]![1] })
  ).toBeVisible();
});
