import { expect, test } from './fixtures';

test('@matrix renders within each strategy viewport without an overflow escape', async ({
  page
}, testInfo) => {
  await page.goto('/');

  const glass = testInfo.project.metadata.glass;
  if (glass === 'off' || glass === 'full') {
    await page.evaluate((level) => {
      document.documentElement.dataset.glass = level;
    }, glass);
  }

  const viewport = page.viewportSize();
  expect(viewport).not.toBeNull();
  await expect(page.locator('body')).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', /^(es|en)$/);

  const dimensions = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
    height: document.documentElement.scrollHeight,
    viewportHeight: window.innerHeight,
    deviceScaleFactor: window.devicePixelRatio
  }));
  expect(dimensions.width).toBeLessThanOrEqual(dimensions.viewportWidth + 1);
  expect(dimensions.height).toBeGreaterThanOrEqual(dimensions.viewportHeight);

  if (viewport?.width === 1100 && viewport.height === 760) {
    expect(dimensions.deviceScaleFactor).toBeGreaterThanOrEqual(1);
  }
});
