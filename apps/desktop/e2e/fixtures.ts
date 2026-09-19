import { expect, test as base } from '@playwright/test';

export const test = base.extend({
  page: async ({ page }, use) => {
    const errors: string[] = [];
    const allowedOrigins = new Set(['http://127.0.0.1:4173']);
    await page.route('**/*', async (route) => {
      const origin = new URL(route.request().url()).origin;
      if (
        origin !== 'http://127.0.0.1:4173' &&
        origin !== 'http://localhost:4173'
      ) {
        throw new Error(`Unexpected external origin: ${origin}`);
      }
      await route.continue();
    });
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    await use(page);
    expect(await page.evaluate(() => localStorage.length)).toBe(0);
    expect(allowedOrigins.size).toBe(1);
    if (errors.length > 0) throw new Error(`E2E errors: ${errors.join('; ')}`);
  }
});

export { expect } from '@playwright/test';
