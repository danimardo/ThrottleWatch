import {
  coverageMatrixSchema,
  liveSnapshotSchema,
  type CoverageMatrix,
  type LiveSnapshot
} from '../lib/bridge/schemas';

export function liveSnapshot(
  overrides: Partial<LiveSnapshot> = {}
): LiveSnapshot {
  return liveSnapshotSchema.parse({
    captured_at_ms: 1_000,
    freshness: 'fresh',
    age_ms: 0,
    temperature_c: 72,
    thermal_limit_c: 95,
    thermal_margin_c: 23,
    load_percent: 90,
    active_clock_mhz: 3_800,
    base_clock_mhz: 3_500,
    package_power_w: 65,
    power_limit_w: 95,
    classification: 'normal',
    severity: null,
    cpu_label: 'CPU replay',
    topology_label: '8 núcleos homogéneos',
    power_label: 'Corriente alterna · Equilibrado',
    collector_state: 'running',
    coverage: {
      tier: 'A',
      confidence_ceiling: 'high',
      advanced_access: 'not_needed'
    },
    confidence_label: 'Confianza máxima alcanzable: alta',
    active_cores: 8,
    platform_kind: null,
    in_turbo_window: false,
    cooling_potential: null,
    guided_result: null,
    ...overrides
  });
}

export function coverage(
  overrides: Partial<CoverageMatrix> = {}
): CoverageMatrix {
  return coverageMatrixSchema.parse({
    tier: 'A',
    confidence_ceiling: 'high',
    advanced_access: 'not_needed',
    conclusion_key: 'coverage.conclusion.a',
    rows: [
      {
        id: 'temperature',
        label: 'Temperatura',
        available: true,
        quality: 'direct',
        quality_label: 'Directa',
        source_label: 'CPU package'
      },
      {
        id: 'active_clock',
        label: 'Frecuencia activa',
        available: true,
        quality: 'derived',
        quality_label: 'Derivada',
        source_label: 'PDH'
      },
      {
        id: 'power',
        label: 'Potencia',
        available: true,
        quality: 'direct',
        quality_label: 'Directa',
        source_label: 'CPU package'
      }
    ],
    ...overrides
  });
}

export type FakeBridgeEvent =
  keyof typeof import('../lib/bridge/schemas').eventSchemas;

export class FakeBridge {
  private readonly listeners = new Map<string, Set<(value: unknown) => void>>();
  private snapshotValue: LiveSnapshot;
  private coverageValue: CoverageMatrix;

  public constructor(
    snapshotValue = liveSnapshot(),
    coverageValue = coverage()
  ) {
    this.snapshotValue = snapshotValue;
    this.coverageValue = coverageValue;
  }

  public getLiveSnapshot(): LiveSnapshot {
    return liveSnapshot(this.snapshotValue);
  }

  public getCoverage(): CoverageMatrix {
    return coverage(this.coverageValue);
  }

  public subscribe(
    event: FakeBridgeEvent,
    listener: (value: unknown) => void
  ): () => void {
    const listeners =
      this.listeners.get(event) ?? new Set<(value: unknown) => void>();
    listeners.add(listener);
    this.listeners.set(event, listeners);
    return () => {
      listeners.delete(listener);
    };
  }

  public emit(event: FakeBridgeEvent, value: unknown): void {
    this.listeners.get(event)?.forEach((listener) => {
      listener(value);
    });
  }
}
