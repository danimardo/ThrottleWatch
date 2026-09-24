import { expect, test } from './fixtures';

/** Simulates the `appearance:glass-effective` push `appearance.rs`'s monitor thread sends live
 * (a preference change, the OS transparency setting changing, or the automatic performance
 * ceiling stepping up/down, T131) — this harness runs against a plain browser tab, so nothing
 * here can drive the real WebView2-CPU-sampling monitor thread. */
async function emitGlassEffective(
  page: import('@playwright/test').Page,
  level: 'full' | 'reduced' | 'off'
) {
  await page.evaluate((glassLevel) => {
    (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__('appearance:glass-effective', {
      level: glassLevel
    });
  }, level);
}

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

test('@a11y glass applies live on Rust’s push, no reload needed (T-E2E-09, T131)', async ({
  page
}) => {
  await page.goto('/');
  // Waits for the shell to actually mount (and so for its appearance:glass-effective listener
  // to be registered) before simulating Rust's push — the same reason every other spec in this
  // suite asserts a landmark is visible before emitting a fixture event.
  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();
  // Default preference is `system`; the frontend's own placeholder resolves that to `full`
  // (no data-glass attribute) until Rust's real answer arrives — exactly the race
  // get_effective_glass_level exists to close, simulated here by the first push.
  await expect(page.locator('html')).not.toHaveAttribute('data-glass');

  await emitGlassEffective(page, 'reduced');
  await expect(page.locator('html')).toHaveAttribute('data-glass', 'reduced');

  await emitGlassEffective(page, 'off');
  await expect(page.locator('html')).toHaveAttribute('data-glass', 'off');

  // Restoring back to `full` clears the attribute rather than setting it to the literal string
  // (tokens.ts::applyGlassLevel, missing attribute = full).
  await emitGlassEffective(page, 'full');
  await expect(page.locator('html')).not.toHaveAttribute('data-glass');
});
