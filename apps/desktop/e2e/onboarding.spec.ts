import AxeBuilder from '@axe-core/playwright';
import { expect, test } from './fixtures';

test('@critical onboarding can be completed with keyboard only', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/?onboarding=fresh');
  await expect(
    page.getByRole('button', { name: /skip|omitir/i })
  ).toBeVisible();

  const next = page.getByRole('button', {
    name: /next|siguiente|start|empezar|open now|abrir ahora/i
  });
  for (let slide = 0; slide < 5; slide += 1) {
    await next.focus();
    await page.keyboard.press('Enter');
  }

  await expect(
    page.getByRole('heading', { name: 'ThrottleWatch' })
  ).toBeVisible();
});

test('@critical @a11y onboarding remains accessible at 480x600 with slide captures', async ({
  page
}) => {
  await page.setViewportSize({ width: 480, height: 600 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/?onboarding=fresh');

  const next = page.getByRole('button', {
    name: /next|siguiente|start|empezar|open now|abrir ahora/i
  });
  for (let slide = 0; slide < 5; slide += 1) {
    const axe = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(
      axe.violations.filter(
        (violation) =>
          violation.impact === 'serious' || violation.impact === 'critical'
      ),
      `onboarding slide ${slide + 1}`
    ).toEqual([]);
    if (slide === 0 || slide === 4) {
      await page.screenshot({
        path: test.info().outputPath(`onboarding-slide-${slide + 1}.png`),
        fullPage: true
      });
    }
    if (slide < 4) {
      await next.focus();
      await page.keyboard.press('Enter');
    }
  }
  await next.focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { name: 'ThrottleWatch' })
  ).toBeVisible();
});

test('@critical onboarding resumes at the saved slide and can be skipped by keyboard', async ({
  page
}) => {
  for (const viewport of [
    { width: 1100, height: 760 },
    { width: 480, height: 600 }
  ]) {
    await page.setViewportSize(viewport);
    await page.goto('/?onboarding=midway');
    await expect(
      page.getByText(
        /continuing where you left off|continuando donde lo dejaste/i
      )
    ).toBeVisible();

    const skip = page.getByRole('button', { name: /skip|omitir/i });
    await skip.focus();
    await page.keyboard.press('Enter');
    await expect(
      page.getByRole('heading', { name: 'ThrottleWatch' })
    ).toBeVisible();
  }
});

test('@critical onboarding explains an advanced-access error instead of showing an empty alert', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/?onboarding=fresh&access=error');

  const next = page.getByRole('button', {
    name: /next|siguiente|start|empezar|open now|abrir ahora/i
  });
  for (let slide = 0; slide < 4; slide += 1) {
    await next.focus();
    await page.keyboard.press('Enter');
  }

  // On the last slide a critical banner shows up for this state. It used to be drawn with no text
  // at all — the shell had no wording for `error`, so a red box with only an icon appeared and
  // nobody could tell what it meant.
  const banners = page.locator('main.onboarding .tw-banner');
  await expect(banners.first()).toBeVisible();
  for (let index = 0; index < (await banners.count()); index += 1) {
    await expect(banners.nth(index)).toHaveText(/\S/);
  }
});
