import { expect, test } from './fixtures';

type Call = { command: string; args?: Record<string, unknown> };

async function calls(page: import('@playwright/test').Page): Promise<Call[]> {
  return page.evaluate(
    () =>
      (window as unknown as { __THROTTLEWATCH_CALLS__: Call[] })
        .__THROTTLEWATCH_CALLS__
  );
}

test('@critical ajustes desactiva el acceso avanzado y vuelve a nivel B/C', async ({
  page
}) => {
  await page.goto('/?access=available');
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();

  await page
    .getByRole('button', {
      name: /disable advanced access|desactivar acceso avanzado/i
    })
    .click();

  await expect(
    page.getByText(/advanced access is off|acceso avanzado está desactivado/i)
  ).toBeVisible();
  expect((await calls(page)).map((call) => call.command)).toContain(
    'disable_advanced_access'
  );
});

test('@critical ajustes ofrece actualizar un PawnIO anterior con la acción upgrade', async ({
  page
}) => {
  await page.goto('/?access=upgradable');
  await page.keyboard.press('Control+6');

  await expect(
    page.getByText(/older version|versión anterior/i).first()
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: /update advanced access|actualizar acceso avanzado/i
    })
    .click();

  await expect
    .poll(
      async () =>
        (await calls(page)).find(
          (call) => call.command === 'request_low_level_access'
        )?.args
    )
    .toEqual({ request: { action: 'upgrade' } });
});

test('@critical ajustes instala con la acción install cuando falta PawnIO', async ({
  page
}) => {
  await page.goto('/?access=installable');
  await page.keyboard.press('Control+6');

  await page
    .getByRole('button', {
      name: /install advanced access|instalar acceso avanzado/i
    })
    .click();

  await expect
    .poll(
      async () =>
        (await calls(page)).find(
          (call) => call.command === 'request_low_level_access'
        )?.args
    )
    .toEqual({ request: { action: 'install' } });
});

test('@critical ajustes ofrece ayuda (no reparar) cuando una política bloquea el acceso (FR-090, data-model.md)', async ({
  page
}) => {
  // `denied` means a policy or antivirus blocks it (data-model.md: "explica que una política o
  // el antivirus lo bloquea; enlace a ayuda") — there is nothing to retry, unlike `error`.
  await page.goto('/?access=denied');
  await page.keyboard.press('Control+6');

  await expect(
    page.getByText(
      /advanced access is off or blocked|acceso avanzado está desactivado o bloqueado/i
    )
  ).toBeVisible();
  await expect(
    page.getByRole('button', {
      name: /repair advanced access|reparar acceso avanzado/i
    })
  ).toHaveCount(0);

  await page
    .getByRole('button', {
      name: /see help about advanced access|ver ayuda sobre el acceso avanzado/i
    })
    .click();
  await expect
    .poll(
      async () =>
        (await calls(page)).find((call) => call.command === 'open_external_url')
          ?.args
    )
    .toEqual({ request: { target: 'help' } });
});

test('@critical ajustes repara con la acción repair cuando el proveedor falla en marcha (FR-090)', async ({
  page
}) => {
  await page.goto('/?access=error');
  await page.keyboard.press('Control+6');

  await expect(
    page.getByText(
      /advanced access stopped responding|acceso avanzado dejó de responder/i
    )
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: /repair advanced access|reparar acceso avanzado/i
    })
    .click();

  await expect
    .poll(
      async () =>
        (await calls(page)).find(
          (call) => call.command === 'request_low_level_access'
        )?.args
    )
    .toEqual({ request: { action: 'repair' } });
});
