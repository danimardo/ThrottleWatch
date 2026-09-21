import { expect, test } from './fixtures';

test('@critical keyboard shortcuts open help and the active session export', async ({
  page
}) => {
  await page.goto('/?shortcut-fixture');
  await expect(
    page.getByRole('heading', { name: 'ThrottleWatch' })
  ).toBeVisible();

  await page.keyboard.press('F1');
  await expect
    .poll(() =>
      page.evaluate(() => {
        const calls = (
          window as unknown as {
            __THROTTLEWATCH_CALLS__: Array<{
              command: string;
              args?: Record<string, unknown>;
            }>;
          }
        ).__THROTTLEWATCH_CALLS__;
        return calls.some(
          ({ command, args }) =>
            command === 'open_external_url' &&
            JSON.stringify(args) ===
              JSON.stringify({ request: { target: 'help' } })
        );
      })
    )
    .toBe(true);

  await page.keyboard.press('Control+E');
  await expect(
    page.getByRole('main', { name: /sessions|sesiones/i })
  ).toBeVisible();
  await expect(page.getByRole('dialog')).toBeVisible();
});
