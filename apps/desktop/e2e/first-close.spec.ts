import { expect, test } from './fixtures';

type Call = { command: string; args?: Record<string, unknown> };

async function commands(page: import('@playwright/test').Page) {
  const calls = await page.evaluate(
    () =>
      (window as unknown as { __THROTTLEWATCH_CALLS__: Call[] })
        .__THROTTLEWATCH_CALLS__
  );
  return calls;
}

/** Simulates what Rust emits on its own the first time it intercepts a close attempt with no
 * `lifecycle.close_action` saved yet (T182, FR-046) — native or from the TitleBar button, both
 * trigger the same `CloseRequested`, see `lib.rs`. This harness runs against a plain browser tab,
 * so nothing here can produce that natively. */
async function emitFirstCloseRequired(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__('lifecycle:close-decision-required', null);
  });
}

test('@critical primer cierre: elegir salir se lo pide a Rust', async ({
  page
}) => {
  await page.goto('/');
  await emitFirstCloseRequired(page);
  await expect(
    page.getByRole('heading', {
      name: /what should happen when you close\?|qué quieres hacer al cerrar\?/i
    })
  ).toBeVisible();

  await page.getByRole('button', { name: /^exit$|^salir$/i }).click();
  const calls = await commands(page);
  const resolved = calls.find((call) => call.command === 'resolve_first_close');
  expect(resolved?.args).toEqual({ request: { action: 'exit' } });
});

test('@critical primer cierre: elegir bandeja se lo pide a Rust', async ({
  page
}) => {
  await page.goto('/');
  await emitFirstCloseRequired(page);
  await page
    .getByRole('button', { name: /keep in the tray|seguir en la bandeja/i })
    .click();
  const calls = await commands(page);
  const resolved = calls.find((call) => call.command === 'resolve_first_close');
  expect(resolved?.args).toEqual({ request: { action: 'tray' } });
});

test('@critical primer cierre: descartar con Esc lo comunica como "dismiss"', async ({
  page
}) => {
  await page.goto('/');
  await emitFirstCloseRequired(page);
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('heading', {
      name: /what should happen when you close\?|qué quieres hacer al cerrar\?/i
    })
  ).toHaveCount(0);
  const calls = await commands(page);
  const resolved = calls.find((call) => call.command === 'resolve_first_close');
  expect(resolved?.args).toEqual({ request: { action: 'dismiss' } });
});
