import { expect, test } from './fixtures';

test('@visual @a11y language, theme and motion preferences apply at the document boundary', async ({
  page
}, testInfo) => {
  await page.emulateMedia({
    colorScheme: 'light',
    reducedMotion: 'reduce',
    forcedColors: 'none'
  });
  await page.goto('/');

  const expectedLocale = testInfo.project.name.endsWith('-es') ? 'es' : 'en';
  await expect(page.locator('html')).toHaveAttribute('lang', expectedLocale);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('html')).not.toHaveAttribute('data-motion');
  await page.screenshot({
    path: testInfo.outputPath(`${expectedLocale}-light.png`),
    fullPage: true
  });

  await page.emulateMedia({
    colorScheme: 'dark',
    reducedMotion: 'no-preference',
    forcedColors: 'active'
  });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('html')).not.toHaveAttribute('data-motion');
  await page.screenshot({
    path: testInfo.outputPath(`${expectedLocale}-dark-forced.png`),
    fullPage: true
  });
});
