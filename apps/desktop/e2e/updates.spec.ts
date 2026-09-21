import { expect, test } from './fixtures';

type Call = { command: string; args?: Record<string, unknown> };

async function commands(page: import('@playwright/test').Page) {
  const calls = await page.evaluate(
    () =>
      (window as unknown as { __THROTTLEWATCH_CALLS__: Call[] })
        .__THROTTLEWATCH_CALLS__
  );
  return calls.map((call) => call.command);
}

async function openSettings(
  page: import('@playwright/test').Page,
  url: string
) {
  await page.goto(url);
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();
}

test('@critical actualizaciones apagadas: no se puede comprobar y no hay ninguna petición', async ({
  page
}) => {
  await openSettings(page, '/');
  const check = page.getByRole('button', { name: /check now|buscar ahora/i });
  await expect(check).toBeDisabled();
  await expect(
    page.getByText(
      /turn on automatic update checks|activa la búsqueda automática/i
    )
  ).toBeVisible();
  const issued = await commands(page);
  expect(issued).not.toContain('check_for_update');
  expect(issued).not.toContain('download_update');
  expect(issued).not.toContain('install_update');
});

test('@critical actualizaciones: buscar, descargar y verificar son gestos separados', async ({
  page
}) => {
  await openSettings(page, '/?updates=on');

  await page.getByRole('button', { name: /check now|buscar ahora/i }).click();
  await expect(page.getByText(/9\.9\.9/).first()).toBeVisible();
  expect(await commands(page)).not.toContain('download_update');

  await page.getByRole('button', { name: /^download$|^descargar$/i }).click();
  await expect(
    page.getByText(/signature verified|firma verificada/i)
  ).toBeVisible();
  expect(await commands(page)).not.toContain('install_update');

  await page.getByRole('button', { name: /^install$|^instalar$/i }).click();
  await expect(
    page.getByText(/installing update|instalando actualización/i).first()
  ).toBeVisible();
  const issued = await commands(page);
  expect(issued.indexOf('check_for_update')).toBeLessThan(
    issued.indexOf('download_update')
  );
  expect(issued.indexOf('download_update')).toBeLessThan(
    issued.indexOf('install_update')
  );
});

test('@critical actualizaciones: una operación en curso bloquea la instalación con su motivo', async ({
  page
}) => {
  await openSettings(page, '/?updates=blocked');
  await page.getByRole('button', { name: /check now|buscar ahora/i }).click();
  await page.getByRole('button', { name: /^download$|^descargar$/i }).click();
  await page.getByRole('button', { name: /^install$|^instalar$/i }).click();

  await expect(
    page.getByText(
      /wait until the guided diagnostic ends|espera a que termine el diagnóstico guiado/i
    )
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /^install$|^instalar$/i })
  ).toBeDisabled();
});
