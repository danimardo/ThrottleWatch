export type Catalog = Readonly<Record<string, unknown>>;
export type CatalogQuery = (key: string) => string;

export function createCatalogQuery(catalog: Catalog): CatalogQuery {
  return (key: string) => {
    const value = key.split('.').reduce<unknown>(readSegment, catalog);
    if (typeof value !== 'string') {
      throw new Error(`Missing catalog key: ${key}`);
    }
    return value;
  };
}

function readSegment(current: unknown, segment: string): unknown {
  return isRecord(current) ? current[segment] : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
