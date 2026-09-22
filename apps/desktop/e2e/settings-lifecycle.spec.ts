import { expect, test } from './fixtures';

test('@critical ajustes conserva el flujo de exportación de una sesión', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/?with-history');
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();

  const exportButton = page
    .getByRole('button', { name: /export|exportar/i })
    .last();
  await expect(exportButton).toBeVisible();
  await exportButton.click();

  await expect(
    page.getByRole('main', { name: /sessions|sesiones/i })
  ).toBeVisible();
  await expect(page.locator('dialog[open]')).toBeVisible();
  await expect(page.getByText('throttlewatch-session.json')).toBeVisible();
});

test('@critical ajustes mantiene las dependencias de bandeja visibles en compacto', async ({
  page
}) => {
  await page.setViewportSize({ width: 480, height: 600 });
  await page.goto('/?with-history');
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();

  const traySwitch = page.getByRole('switch', {
    name: /background monitoring|monitorización en segundo plano/i
  });
  await expect(traySwitch).toBeVisible();
  await expect(
    page.getByText(
      /enable tray monitoring first|activa primero la monitorización/i
    )
  ).toHaveCount(0);
  await traySwitch.click();
  await expect(traySwitch).toBeChecked();
});

test('@critical ajustes separa borrar datos de restablecer la aplicación', async ({
  page
}) => {
  await page.goto('/?with-history');
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();

  await page.getByRole('button', { name: /delete data|borrar datos/i }).click();
  await expect(page.locator('dialog[open] h2')).toHaveText(
    /delete data|borrar datos/i
  );
  await page
    .locator('dialog[open]')
    .getByRole('button', { name: /cancel|cancelar/i })
    .click();

  await page
    .getByRole('button', {
      name: /reset throttlewatch|restablecer throttlewatch/i
    })
    .click();
  await expect(page.locator('dialog[open] h2')).toHaveText(
    /reset throttlewatch|restablecer throttlewatch/i
  );
});

test('@critical una sesión activa no expone la acción de borrado', async ({
  page
}) => {
  await page.goto('/?active-session');
  await page.keyboard.press('Control+4');
  await expect(
    page.getByRole('main', { name: /sessions|sesiones/i })
  ).toBeVisible();
  await expect(
    page.locator('.status-tag').filter({ hasText: /in progress|en curso/i })
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /delete session|eliminar sesión/i })
  ).toHaveCount(0);
});

test('@critical ajustes ofrece exportar la base de datos anterior solo si se recuperó una dañada (FR-075)', async ({
  page
}) => {
  await page.goto('/?with-history');
  await page.keyboard.press('Control+6');
  await expect(
    page.getByRole('main', { name: /settings|ajustes/i })
  ).toBeVisible();
  await expect(page.getByText(/dañada|corrupt/i)).toHaveCount(0);

  await page.goto('/?with-history&corrupt-backup');
  await page.keyboard.press('Control+6');
  const corruptRow = page.locator('.row', { hasText: /dañada|corrupt/i });
  await expect(corruptRow).toBeVisible();
  await expect(
    corruptRow.getByText('throttlewatch.db.corrupt-2026-09-22T00-00-00Z')
  ).toBeVisible();
  await corruptRow.getByRole('button', { name: /export|exportar/i }).click();
});
