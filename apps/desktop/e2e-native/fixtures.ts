import {
  test as base,
  chromium,
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
