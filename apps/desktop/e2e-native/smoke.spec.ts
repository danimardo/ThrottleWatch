import { expect, test } from './fixtures';

test('@native the real Tauri window serves the app and responds to a real IPC round trip', async ({
  page
}) => {
  await expect(page).toHaveTitle('ThrottleWatch');
  await expect(page.locator('body')).not.toBeEmpty();

  // `__TAURI_INTERNALS__` is only present inside a real Tauri webview — confirms this page is
  // the native window, not a Vite preview tab a stray `frontend`/`app` run left behind.
  const isRealTauriWindow = await page.evaluate(
    () => '__TAURI_INTERNALS__' in window
  );
  expect(isRealTauriWindow).toBe(true);

  // A real navigation exercises a real IPC round trip (`set_window_state`/`get_preferences` and
  // friends), not the fake bridge `e2e/fixtures.ts` injects for `frontend`/`app`.
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();
});
