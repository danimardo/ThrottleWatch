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

  // The report says what it found in words: the cooling estimate as a range, the limit event by
  // name and how long it lasted, and how much to trust it. It used to print the raw JSON of the
  // cooling potential and the internal event code.
  await expect(page.getByText(/\+5 % (to|y) \+15 %/)).toBeVisible();
  await expect(
    page.getByText(/(thermal limit|limitación térmica) · 12 s/i)
  ).toBeVisible();
  await expect(
    page.getByText(/(confidence in the result|confianza en el resultado)/i)
  ).toBeVisible();
  await expect(page.getByText(/low_percent|"method"/)).toHaveCount(0);

  // What the class means, in words: what was observed, what cannot be concluded and what to do
  // (never "improve cooling" for a power limit), and the causal chain when the reasons are direct.
  await expect(
    page.getByText(
      /reported its thermal limit|indicó límite térmico durante buena parte/i
    )
  ).toBeVisible();
  await expect(
    page.getByText(/check the cooling|revisa la refrigeración/i)
  ).toBeVisible();
  await expect(page.getByText(/^(temperature|temperatura)$/i)).toBeVisible();

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
