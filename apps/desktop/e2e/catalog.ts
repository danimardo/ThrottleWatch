import type { Locator, Page } from '@playwright/test';

export function t(
  page: Page,
  catalog: Readonly<Record<string, unknown>>,
  key: string
): Locator {
  const value = key.split('.').reduce<unknown>(readSegment, catalog);
  if (typeof value !== 'string') {
    throw new Error(`Missing catalog key: ${key}`);
  }
  return page.getByText(value, { exact: true });
}

function readSegment(current: unknown, segment: string): unknown {
  return isRecord(current) ? current[segment] : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
