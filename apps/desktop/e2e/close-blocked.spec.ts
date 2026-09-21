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

/** Simulates what Rust would emit on its own when it intercepts a close attempt (native or from
 * the TitleBar button — both trigger the same `CloseRequested`, see `lib.rs`): this harness runs
 * against a plain browser tab, so nothing here can produce that natively (T090/T-E2E-08). */
async function emitCloseBlocked(
  page: import('@playwright/test').Page,
  reason: 'guided' | 'export' | 'download' | 'install'
) {
  await page.evaluate((reason) => {
    (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__('lifecycle:close-blocked', { reason });
  }, reason);
}

test('@critical cierre bloqueado: diagnóstico guiado detiene la prueba al confirmar', async ({
  page
}) => {
  await page.goto('/');
  await emitCloseBlocked(page, 'guided');
  await expect(
    page.getByText(/guided diagnostic in progress|diagnóstico guiado en curso/i)
  ).toBeVisible();

  await page
    .getByRole('button', { name: /stop and exit|detener y salir/i })
    .click();
  const calls = await commands(page);
  const confirm = calls.find((call) => call.command === 'confirm_close');
  expect(confirm?.args).toEqual({ request: { stop_operation: true } });
});

test('@critical cierre bloqueado: exportación o importación cancela al confirmar', async ({
  page
}) => {
  await page.goto('/');
  await emitCloseBlocked(page, 'export');
  await expect(
    page.getByRole('heading', {
      name: /export or import in progress|exportación o importación en curso/i
    })
  ).toBeVisible();

  await page
    .getByRole('button', { name: /cancel and close|cancelar y cerrar/i })
    .click();
  const calls = await commands(page);
  const confirm = calls.find((call) => call.command === 'confirm_close');
  expect(confirm?.args).toEqual({ request: { stop_operation: true } });
});

test('@critical cierre bloqueado: una descarga se abandona sin cancelarla explícitamente', async ({
  page
}) => {
  await page.goto('/');
  await emitCloseBlocked(page, 'download');
  await expect(
    page.getByText(/downloading an update|descargando una actualización/i)
  ).toBeVisible();

  await page
    .getByRole('button', { name: /close anyway|cerrar de todas formas/i })
    .click();
  const calls = await commands(page);
  const confirm = calls.find((call) => call.command === 'confirm_close');
  expect(confirm?.args).toEqual({ request: { stop_operation: false } });
});

test('@critical cierre bloqueado: una instalación no ofrece ninguna confirmación', async ({
  page
}) => {
  await page.goto('/');
  await emitCloseBlocked(page, 'install');
  await expect(
    page.getByText(/installing an update|instalando una actualización/i)
  ).toBeVisible();
  await expect(
    page.getByRole('button', {
      name: /cancel and close|cancelar y cerrar|close anyway|cerrar de todas formas/i
    })
  ).toHaveCount(0);

  await page.getByRole('button', { name: /got it|entendido/i }).click();
  const calls = await commands(page);
  expect(calls.some((call) => call.command === 'confirm_close')).toBe(false);
});
