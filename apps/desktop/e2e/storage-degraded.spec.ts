import { expect, test } from './fixtures';

/** Simulates what Rust emits on its own when a storage write fails or, later, when the backlog
 * fully drains (FR-075, `WriteBacklog`). This harness runs against a plain browser tab, so
 * nothing here can produce a real full-disk write failure. */
async function emit(page: import('@playwright/test').Page, event: string) {
  await page.evaluate((eventName) => {
    (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__(eventName, null);
  }, event);
}

test('@critical un fallo de almacenamiento avisa de forma persistente y el aviso desaparece al recuperarse (FR-075)', async ({
  page
}) => {
  await page.goto('/');
  await expect(
    page.getByText(/can't save to disk|no se puede guardar en disco/i)
  ).toHaveCount(0);

  await emit(page, 'storage:degraded');
  await expect(
    page.getByText(/can't save to disk|no se puede guardar en disco/i)
  ).toBeVisible();

  await emit(page, 'storage:recovered');
  await expect(
    page.getByText(/can't save to disk|no se puede guardar en disco/i)
  ).toHaveCount(0);
});
