import { expect, test } from './fixtures';

test('@a11y title bar controls and double click keep native semantics', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/');

  const maximize = page.getByRole('button', { name: /maximize|maximizar/i });
  const restore = page.getByRole('button', { name: /restore|restaurar/i });
  await expect(maximize).toBeVisible();
  await maximize.click();
  await expect(restore).toBeVisible();

  const dragRegion = page.locator('[data-tauri-drag-region]');
  await dragRegion.dblclick();
  await expect(maximize).toBeVisible();

  await page.setViewportSize({ width: 480, height: 600 });
  await expect(page.getByRole('button', { name: /more|más/i })).toBeVisible();
});

test('@a11y reduced-motion preference is represented at the document boundary', async ({
  page
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('html')).not.toHaveAttribute('data-motion');
  await expect(
    page.evaluate(
      () => window.matchMedia('(prefers-reduced-motion: reduce)').matches
    )
  ).resolves.toBe(true);
});
