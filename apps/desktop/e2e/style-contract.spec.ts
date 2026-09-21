import { expect, test } from './fixtures';

test('@visual style contract keeps tokens, themes and glass modes observable', async ({
  page
}) => {
  await page.goto('/');

  const contract = await page.evaluate(() => {
    const root = document.documentElement;
    const read = (property: string) =>
      getComputedStyle(root).getPropertyValue(property).trim();

    root.dataset.theme = 'light';
    const lightSurface = read('--surface');
    const lightGlass = read('--glass-bg');
    root.dataset.theme = 'dark';
    const darkSurface = read('--surface');
    root.dataset.glass = 'off';
    const glassOff = {
      filter: read('--glass-filter'),
      background: read('--glass-bg'),
      shadow: read('--glass-shadow')
    };
    delete root.dataset.glass;
    const glassFull = {
      filter: read('--glass-filter'),
      background: read('--glass-bg'),
      shadow: read('--glass-shadow')
    };
    const interactive = Array.from(
      document.querySelectorAll('button, [role="button"]')
    ).map((element) => {
      const style = getComputedStyle(element);
      return {
        cursor: style.cursor,
        decoration: style.textDecorationLine
      };
    });
    delete root.dataset.theme;
    return {
      lightSurface,
      darkSurface,
      lightGlass,
      glassOff,
      glassFull,
      interactive
    };
  });

  expect(contract.lightSurface).not.toBe('');
  expect(contract.darkSurface).not.toBe('');
  expect(contract.lightSurface).not.toBe(contract.darkSurface);
  expect(contract.lightGlass).not.toBe('');
  expect(contract.glassOff.filter).toBe('none');
  expect(contract.glassOff.shadow).toBe('none');
  expect(contract.glassFull.filter).not.toBe('none');
  expect(contract.interactive.every(({ cursor }) => cursor !== 'pointer')).toBe(
    true
  );
  expect(
    contract.interactive.every(({ decoration }) => decoration === 'none')
  ).toBe(true);
});
