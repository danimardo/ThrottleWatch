import { expect, test } from './fixtures';

test('@critical guided diagnosis shows phases and stops with Ctrl+Shift+X', async ({
  page
}) => {
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await expect(
    page.getByRole('heading', { name: /diagnóstico guiado/i })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Iniciar' }).click();
  await expect(
    page.getByRole('button', { name: 'Omitir reposo' })
  ).toBeVisible();
  await page.keyboard.press('Control+Shift+X');
  await expect(page.getByText('Prueba incompleta')).toBeVisible();
});

test('@critical guided session remains stoppable after navigation and protects close', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: 'Iniciar' }).click();
  await page.getByRole('button', { name: 'Omitir reposo' }).click();

  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  const globalStop = page.getByRole('button', { name: 'Detener ahora' });
  await expect(globalStop).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(globalStop).toBeVisible();

  await page.locator('button.close').click();
  await expect(
    page.getByRole('button', { name: 'Detener y salir' })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Cancelar' }).click();
  await expect(
    page.getByRole('button', { name: 'Detener y salir' })
  ).toBeHidden();

  await globalStop.click();
  await expect(globalStop).toBeHidden();
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await expect(page.getByText('Prueba incompleta')).toBeVisible();
});

test('@critical guided diagnosis also works at compact size', async ({
  page
}) => {
  await page.setViewportSize({ width: 480, height: 600 });
  await page.goto('/');
  await page.getByRole('button', { name: /more|más/i }).click();
  await page
    .getByRole('menuitem', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: 'Iniciar' }).click();
  await expect(
    page.getByRole('button', { name: 'Omitir reposo' })
  ).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('button', { name: 'Omitir reposo' })
  ).toBeVisible();
  await page.keyboard.press('Control+Shift+X');
  await expect(page.getByText('Prueba incompleta')).toBeVisible();
});

test('@critical analysis renders four tracks, gaps and event evidence', async ({
  page
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  await expect(
    page.getByRole('group', { name: 'Pistas del análisis' }).first()
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Thermal limit' })
  ).toBeVisible();
  await expect(
    page.getByRole('slider', { name: 'Cursor temporal' })
  ).toHaveAttribute('aria-valuenow', '0');
  await expect(page.locator('path[stroke-dasharray="3 3"]')).toHaveCount(4);
  await expect(page.getByText('Evidencia', { exact: true })).toBeVisible();
  await page.getByRole('slider', { name: 'Cursor temporal' }).press('Home');
  await page.getByRole('slider', { name: 'Cursor temporal' }).press('Enter');
  await page.getByRole('slider', { name: 'Cursor temporal' }).press('End');
  await page.getByRole('slider', { name: 'Cursor temporal' }).press('Enter');
  await expect(
    page.getByText('Rango seleccionado', { exact: true })
  ).toBeVisible();
  await expect(page.locator('table')).toBeAttached();
  await expect(
    page.getByRole('slider', { name: 'Cursor temporal' })
  ).toHaveAttribute('aria-valuenow', '3');
  await page.screenshot({
    path: 'test-results/analysis-guided.png',
    fullPage: true
  });
});

test('@visual analysis benchmark renders four 3000-point tracks', async ({
  page
}) => {
  await page.goto('/?benchmark=analysis');
  const started = Date.now();
  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  await expect(page.getByRole('button', { name: 'temperature' })).toBeVisible();
  const elapsedMs = Date.now() - started;
  await expect(
    page.getByRole('button', { name: 'Thermal limit' })
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Power cap' })).toBeVisible();
  await expect(page.locator('svg path')).not.toHaveCount(0);
  await test.info().attach('analysis-chart-benchmark', {
    body: JSON.stringify({
      tracks: 4,
      points_per_track: 3000,
      elapsed_ms: elapsedMs
    }),
    contentType: 'application/json'
  });
  expect(elapsedMs).toBeLessThan(2_000);
});

test('@critical CPU topology keeps core selection while changing metric', async ({
  page
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /cpu/i }).click();
  await expect(page.getByRole('heading', { name: 'CPU' })).toBeVisible();
  await expect(page.locator('.group-label', { hasText: 'P' })).toBeVisible();
  await expect(page.locator('.group-label', { hasText: 'E' })).toBeVisible();

  const firstCore = page.locator('button[aria-label*="Núcleo 0"]').first();
  await firstCore.click();
  await expect(firstCore).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Frecuencia' }).click();
  await expect(firstCore).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByText('3900 MHz', { exact: true })).toBeVisible();
});
