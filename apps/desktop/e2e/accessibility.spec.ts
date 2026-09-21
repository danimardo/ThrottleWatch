import AxeBuilder from '@axe-core/playwright';
import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

async function expectNoSeriousViolations(page: Page) {
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa'])
    .analyze();
  expect(
    result.violations.filter(
      (violation) =>
        violation.impact === 'serious' || violation.impact === 'critical'
    )
  ).toEqual([]);
}

test('@a11y foundation has no serious or critical axe violations', async ({
  page
}) => {
  await page.goto('/');
  await expectNoSeriousViolations(page);
});

test('@a11y forced colors and reduced motion preserve the accessibility contract', async ({
  page
}) => {
  await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('html')).not.toHaveAttribute('data-motion');
  await expectNoSeriousViolations(page);
});

test('@a11y every primary screen passes the serious/critical axe gate', async ({
  page
}) => {
  test.setTimeout(180_000);
  const screens = [
    { key: '1', name: /now|ahora/i },
    { key: '2', name: /analysis|análisis/i },
    { key: '3', name: /cpu/i },
    { key: '4', name: /sessions|sesiones/i },
    { key: '5', name: /guided|guiado/i },
    { key: '6', name: /settings|ajustes/i }
  ];

  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' });
    for (const viewport of [
      { width: 480, height: 600 },
      { width: 840, height: 760 },
      { width: 1100, height: 760 }
    ]) {
      await page.setViewportSize(viewport);
      await page.goto('/?with-history');
      for (const screen of screens) {
        await page.keyboard.press(`Control+${screen.key}`);
        await expect(
          page.getByRole('main', { name: screen.name })
        ).toBeVisible();
        await expectNoSeriousViolations(page);
      }
    }
  }
});
