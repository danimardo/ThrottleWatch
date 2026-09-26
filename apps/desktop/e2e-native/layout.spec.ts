import {
  ensureOnboardingDone,
  expect,
  openScreen,
  tauriInvoke,
  test
} from './fixtures';

/**
 * Layout the fake bridge could not check, because it needs the real shell around the real screens.
 */
test('@native the "What is new" notice lines up with the cards under it', async ({
  page
}) => {
  await ensureOnboardingDone(page);

  // Bring the notice back: it is dismissed for good once acknowledged, and the application is shared.
  const current = await tauriInvoke<Record<string, unknown>>(
    page,
    'get_onboarding_state'
  );
  await tauriInvoke(page, 'set_onboarding_state', {
    request: { ...current, last_seen_notice_version: 0 }
  });
  await page.reload();
  await ensureOnboardingDone(page);
  await openScreen(page, 'Control+1', /now|ahora/i);

  const notice = page.locator('.tw-whats-new');
  await expect(notice).toBeVisible();
  const strip = page.locator('.tw-context-strip').first();
  await expect(strip).toBeVisible();

  const [noticeBox, stripBox] = await Promise.all([
    notice.boundingBox(),
    strip.boundingBox()
  ]);
  expect(noticeBox).not.toBeNull();
  expect(stripBox).not.toBeNull();
  // Same left and right edge: the notice used to run to the window edge, 24 px wider on each side.
  expect(Math.abs((noticeBox?.x ?? 0) - (stripBox?.x ?? 0))).toBeLessThan(1);
  expect(
    Math.abs(
      (noticeBox?.x ?? 0) +
        (noticeBox?.width ?? 0) -
        ((stripBox?.x ?? 0) + (stripBox?.width ?? 0))
    )
  ).toBeLessThan(1);
});
