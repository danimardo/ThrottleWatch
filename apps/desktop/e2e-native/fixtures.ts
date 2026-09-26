import {
  test as base,
  chromium,
  expect,
  type Browser,
  type Page
} from '@playwright/test';

/**
 * Connects to the real Tauri window `global-setup.ts` already launched, instead of Playwright
 * launching its own browser — the whole point of T-PLAY-002 is exercising that window, not a
 * fresh Chromium instance.
 */
export const test = base.extend<{ page: Page }, { nativeBrowser: Browser }>({
  nativeBrowser: [
    async ({}, use) => {
      const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
      await use(browser);
      // `close()` on a CDP-attached browser only detaches Playwright; it does not quit the real
      // app (`global-setup.ts`'s teardown does that).
      await browser.close();
    },
    { scope: 'worker' }
  ],
  page: async ({ nativeBrowser }, use) => {
    const context =
      nativeBrowser.contexts()[0] ?? (await nativeBrowser.newContext());
    const page = context.pages()[0] ?? (await context.newPage());
    await use(page);
  }
});

export { expect } from '@playwright/test';

/**
 * Waits for the application to be past onboarding, dismissing it when it is showing.
 *
 * T181: the suite starts from an empty database (`TW_DEV_DATA_DIR`), so the first thing on screen is
 * onboarding, which swallows the navigation shortcuts. The tests used to dismiss it with
 * `skip.isVisible()`, which answers instantly and does not wait for anything to render: fine on a
 * fast machine, but on the CI runner the button was not painted yet, nothing was clicked, and every
 * test then failed looking for a `main` landmark (measured 2026-09-26: the same suite passed one
 * run and failed 3/3 the next).
 *
 * So this waits for whichever comes first: the skip button, or the application's own `main`. The
 * onboarding has a `main` landmark too (`.onboarding`, "Introducción inicial"), so it must be
 * excluded explicitly or "the app is ready" would be true while onboarding is still up.
 */
export async function ensureOnboardingDone(page: Page): Promise<void> {
  const skip = page.getByRole('button', { name: /omitir|skip/i }).first();
  const app = page.locator('main:not(.onboarding)').first();
  await expect(skip.or(app)).toBeVisible({ timeout: 30_000 });
  if (await skip.isVisible()) {
    await skip.click();
  }
  await expect(app).toBeVisible({ timeout: 30_000 });
}
