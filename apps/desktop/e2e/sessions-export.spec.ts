import { expect, test } from './fixtures';

test('@critical sesiones abre informe, referencia y confirmación de borrado', async ({
  page
}) => {
  await page.goto('/?with-history');
  await page.keyboard.press('Control+4');
  await expect(
    page.getByRole('main', { name: /sessions|sesiones/i })
  ).toBeVisible();

  await page.getByRole('button', { name: /open|abrir/i }).click();
  await expect(
    page.getByRole('button', { name: /back to sessions|volver a sesiones/i })
  ).toBeVisible();

  await page
    .getByRole('button', { name: /back to sessions|volver a sesiones/i })
    .click();
  const reference = page.getByRole('button', {
    name: /use as reference|usar como referencia/i
  });
  await expect(reference).toBeVisible();
  await reference.click();

  await page
    .getByRole('button', { name: /delete session|eliminar sesión/i })
    .click();
  await expect(page.locator('dialog[open]')).toBeVisible();
  await page
    .locator('dialog[open]')
    .getByRole('button', { name: /delete|eliminar/i })
    .click();

  const calls = await page.evaluate(
    () =>
      (
        window as unknown as {
          __THROTTLEWATCH_CALLS__: Array<{ command: string }>;
        }
      ).__THROTTLEWATCH_CALLS__
  );
  expect(calls.map(({ command }) => command)).toEqual(
    expect.arrayContaining([
      'get_session',
      'set_session_reference',
      'delete_session'
    ])
  );
});

test('@critical sesiones previsualiza, anonimiza, exporta e importa', async ({
  page
}) => {
  await page.goto('/?with-history');
  await page.keyboard.press('Control+4');
  await page.getByRole('button', { name: /open|abrir/i }).click();
  await page
    .getByRole('button', { name: /export session|exportar sesión/i })
    .click();
  await expect(page.locator('dialog[open]')).toBeVisible();
  await expect(page.getByText('throttlewatch-session.json')).toBeVisible();
  await page.getByRole('button', { name: /save|guardar/i }).click();

  await page
    .getByRole('button', { name: /back to sessions|volver a sesiones/i })
    .click();
  await page
    .getByRole('button', { name: /import session|importar sesión/i })
    .click();
  await expect(
    page.getByText(/session imported|sesión importada/i)
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /open session|abrir sesión/i })
  ).toBeVisible();

  await page
    .getByRole('button', { name: /open session|abrir sesión/i })
    .click();
  await expect(
    page.getByText(/reevaluation with today|reevaluación con las reglas/i)
  ).toBeVisible();
  await expect(
    page.getByText(
      /would be classified differently|se clasificaría como distinto/i
    )
  ).toBeVisible();

  const calls = await page.evaluate(
    () =>
      (
        window as unknown as {
          __THROTTLEWATCH_CALLS__: Array<{ command: string }>;
        }
      ).__THROTTLEWATCH_CALLS__
  );
  expect(calls.map(({ command }) => command)).toEqual(
    expect.arrayContaining(['import_session', 'reevaluate_report'])
  );
});
