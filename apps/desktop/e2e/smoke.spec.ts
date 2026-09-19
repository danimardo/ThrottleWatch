import { test, expect } from './fixtures';

test('@smoke foundation page has no unexpected storage', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'ThrottleWatch' })
  ).toBeVisible();
  expect(errors).toEqual([]);
  expect(await page.evaluate(() => localStorage.length)).toBe(0);
});
