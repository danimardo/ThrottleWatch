import { expect, test } from './fixtures';

const destinations = [
  { shortcut: '1', name: /now|ahora/i },
  { shortcut: '2', name: /analysis|análisis/i },
  { shortcut: '3', name: /cpu/i },
  { shortcut: '4', name: /sessions|sesiones/i },
  { shortcut: '5', name: /guided diagnostic|diagnóstico guiado/i },
  { shortcut: '6', name: /settings|ajustes/i }
] as const;

test('@critical compact navigation and shortcuts work at supported widths', async ({
  page
}) => {
  for (const viewport of [
    { width: 480, height: 600 },
    { width: 480, height: 500 },
    { width: 840, height: 760 }
  ]) {
    await page.setViewportSize(viewport);
    await page.goto('/');

    if (viewport.width < 700) {
      await page.getByRole('button', { name: /more|más/i }).click();
      for (const destination of destinations.slice(3)) {
        await expect(
          page.getByRole('menuitem', { name: destination.name })
        ).toBeVisible();
      }
      await page.keyboard.press('Escape');
    } else {
      await expect(
        page.getByRole('navigation', { name: /destinations|destinos/i })
      ).toBeVisible();
    }

    for (const destination of destinations) {
      await page.keyboard.press(`Control+${destination.shortcut}`);
      await expect(
        page.getByRole('main', { name: destination.name })
      ).toBeVisible();
    }
    await page.keyboard.press('Control+,');
    await expect(
      page.getByRole('main', { name: /settings|ajustes/i })
    ).toBeVisible();
  }
});
