import type { Page } from '@playwright/test';
import {
  ensureOnboardingDone,
  expect,
  openScreen,
  tauriInvoke,
  test
} from './fixtures';

/**
 * T181: from `e2e/guided-analysis.spec.ts` (first two tests), against the real application: the
 * guided diagnostic really runs its preflight (sensors, AC power, disk space, load generator),
 * really starts a session and really writes to the database, instead of a fake `start_guided` that
 * always answers "rest phase".
 *
 * Not migrated, deliberately: the compact-size variant and the 3000-point benchmark are viewport
 * and rendering-performance questions.
 */
const GUIDED = /guided diagnostic|diagnóstico guiado/i;
const SKIP_REST = /skip rest|omitir reposo/i;
const INCOMPLETE = /incomplete test|prueba incompleta/i;
const START = /start|iniciar/i;

/**
 * The application under test is one instance shared by every spec, and the guided screen keeps its
 * outcome on screen after a test ("Try again"/"Close" instead of "Start") until the window is
 * reloaded. The fake bridge never had this problem — each `goto` was a fresh application — so a
 * guided scenario starts by reloading, and does not depend on which one ran before it.
 */
test.afterEach(async ({ page }) => {
  // A guided scenario that fails midway leaves a test *running* in the one application every spec
  // shares, and everything after it then fails against an application in a state nobody chose: on
  // the CI runner one failure here (stopping took longer than the default 5 s) was followed by 12
  // more, all 30 s timeouts. Stop whatever is running, and reload so no dialog or half-open screen
  // survives, whatever way the scenario ended.
  await tauriInvoke(page, 'stop_guided').catch(() => undefined);
  await page.reload();
});

async function openFreshGuidedScreen(page: Page): Promise<void> {
  await ensureOnboardingDone(page);
  await page.reload();

  // Wait for what the application itself says it needs, not for a fixed time. The guided test's
  // preflight asks whether the collector is delivering temperature and an active clock *right now*,
  // and the first scenario of the suite runs the instant the application starts: on the CI runner
  // it clicked "Start" before any telemetry had arrived and the preflight refused (6.3 s, no "skip
  // rest"), while the very same steps passed in the next scenario a few seconds later. A fast
  // developer machine always had the data in time, so it could never show. Polling the real
  // `get_guided_preflight` makes the start conditional on the same fact the application checks.
  await expect
    .poll(
      async () =>
        (await tauriInvoke<{ sensors: boolean }>(page, 'get_guided_preflight'))
          .sensors,
      {
        timeout: 30_000,
        message: 'the collector never delivered temperature and an active clock'
      }
    )
    .toBe(true);

  await openScreen(page, 'Control+5', GUIDED);
  await expect(page.getByRole('button', { name: START })).toBeVisible();
}

test('@native guided diagnosis shows phases and stops with Ctrl+Shift+X', async ({
  page
}) => {
  await openFreshGuidedScreen(page);

  await page.getByRole('button', { name: START }).click();
  await expect(page.getByRole('button', { name: SKIP_REST })).toBeVisible();

  await page.keyboard.press('Control+Shift+X');
  await expect(page.getByText(INCOMPLETE)).toBeVisible();
});

test('@native a guided session stays stoppable after navigation and protects close', async ({
  page
}) => {
  await openFreshGuidedScreen(page);
  await page.getByRole('button', { name: START }).click();
  await page.getByRole('button', { name: SKIP_REST }).click();

  // Leave the guided screen: the session keeps running and the global stop follows the person.
  await openScreen(page, 'Control+2', /analysis|análisis/i);
  const globalStop = page.getByRole('button', {
    name: /stop now|detener ahora/i
  });
  await expect(globalStop).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(globalStop).toBeVisible();

  // Closing the real window must be intercepted by Rust (T090) while a guided test is running,
  // not obeyed: the dialog offers to stop first, and cancelling leaves everything as it was.
  const stopAndExit = page.getByRole('button', {
    name: /stop and exit|detener y salir/i
  });
  await page.locator('button.close').click();
  await expect(stopAndExit).toBeVisible();
  await page.getByRole('button', { name: /cancel|cancelar/i }).click();
  await expect(stopAndExit).toBeHidden();
  await expect(globalStop).toBeVisible();

  // Stopping joins the load generator's threads, which took longer than the 5 s default on the CI
  // runner (a VM with few cores).
  await globalStop.click();
  await expect(globalStop).toBeHidden({ timeout: 30_000 });
  await openScreen(page, 'Control+5', GUIDED);
  await expect(page.getByText(INCOMPLETE)).toBeVisible();
});
