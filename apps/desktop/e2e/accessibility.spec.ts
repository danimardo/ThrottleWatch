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

test('@a11y glass off and the no-backdrop-filter fallback both preserve AA contrast (T131)', async ({
  page
}) => {
  // `off` (an explicit choice, or T131's automatic performance degradation landing on it):
  // solid surfaces, no blur, no ambient animation — axe's color-contrast rule (wcag2aa) covers
  // exactly the case tokens.css's `[data-glass='off']` block exists for.
  await page.goto('/?glass=off');
  await expect(page.locator('html')).toHaveAttribute('data-glass', 'off');
  await expectNoSeriousViolations(page);

  // tokens.css's `@supports not (backdrop-filter)` fallback (old WebView2) sets the same
  // near-solid alpha values without a browser that actually lacks the feature — Chromium always
  // supports it, so this reproduces the fallback's own CSS custom properties directly instead of
  // trying to fake feature detection.
  await page.goto('/');
  await page.addStyleTag({
    content:
      ':root { --glass-alpha: 0.94; --glass-alpha-strong: 0.97; --glass-alpha-subtle: 0.85; }'
  });
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
