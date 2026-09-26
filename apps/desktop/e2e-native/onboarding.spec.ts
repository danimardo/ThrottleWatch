import {
  ensureOnboardingDone,
  expect,
  openScreen,
  resetOnboarding,
  test
} from './fixtures';

/**
 * T181: from `e2e/onboarding.spec.ts`, against the real application. The fake bridge decided the
 * slide with `?onboarding=fresh|midway`; here the state is written through the real
 * `set_onboarding_state` and read back by the real `get_onboarding_state`, so the persistence the
 * "resumes where you left off" scenario is about is actually exercised.
 *
 * Not migrated, deliberately: the 480x600 axe/screenshot scenario is viewport and CSS, which a real
 * window cannot be resized to over CDP.
 */
const NEXT = /next|siguiente|start|empezar|open now|abrir ahora/i;
const SKIP = /skip|omitir/i;

test('@native onboarding can be completed with keyboard only', async ({
  page
}) => {
  await resetOnboarding(page, 1);
  await expect(page.getByRole('button', { name: SKIP })).toBeVisible();

  const next = page.getByRole('button', { name: NEXT });
  for (let slide = 0; slide < 5; slide += 1) {
    await next.focus();
    await page.keyboard.press('Enter');
  }

  // Past onboarding the application's own `main` is on screen, not the onboarding one.
  await expect(page.locator('main:not(.onboarding)').first()).toBeVisible();
  await expect(page.locator('main.onboarding')).toHaveCount(0);
});

test('@native onboarding resumes at the saved slide and can be skipped by keyboard', async ({
  page
}) => {
  await resetOnboarding(page, 3);
  await expect(
    page.getByText(
      /continuing where you left off|continuando donde lo dejaste/i
    )
  ).toBeVisible();

  const skip = page.getByRole('button', { name: SKIP });
  await skip.focus();
  await page.keyboard.press('Enter');

  await expect(page.locator('main:not(.onboarding)').first()).toBeVisible();
  await expect(page.locator('main.onboarding')).toHaveCount(0);
});

test('@native the introduction owns the whole window: no sidebar until it is over', async ({
  page
}) => {
  await resetOnboarding(page, 1);
  await expect(page.locator('main.onboarding')).toBeVisible();

  // No sidebar (nor bottom bar) while it is showing, on any of its slides...
  const sidebar = page.getByRole('navigation');
  await expect(sidebar).toHaveCount(0);
  const next = page.getByRole('button', { name: NEXT });
  for (let slide = 0; slide < 4; slide += 1) {
    await next.focus();
    await page.keyboard.press('Enter');
    await expect(sidebar).toHaveCount(0);
  }

  // ...and it comes back on leaving, with the person on "Now", not on the screen they were on when
  // they asked to repeat the introduction.
  await next.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('main.onboarding')).toHaveCount(0);
  await expect(sidebar.first()).toBeVisible();
  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();
});

test('@native leaving the introduction from Settings lands on Now', async ({
  page
}) => {
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+6', /settings|ajustes/i);
  await page
    .getByRole('button', { name: /repeat introduction|repetir introducción/i })
    .click();
  await expect(page.locator('main.onboarding')).toBeVisible();

  await page.getByRole('button', { name: SKIP }).focus();
  await page.keyboard.press('Enter');

  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();
  // The highlighted destination is the one on screen. "Now" is the current page in the sidebar.
  await expect(
    page.getByRole('navigation').first().locator('[aria-current="page"]')
  ).toHaveCount(1);
});
