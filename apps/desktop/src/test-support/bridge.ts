import {
  coverageMatrixSchema,
  liveSnapshotSchema,
  onboardingStateSchema,
  type CoverageMatrix,
  type LiveSnapshot,
  type OnboardingState
} from '../lib/bridge/schemas';
import type { BridgeTransport } from '../lib/bridge';
import { z } from 'zod';

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

export function onboardingState(
  overrides: Partial<OnboardingState> = {}
): OnboardingState {
  return onboardingStateSchema.parse({
    flow_version: 1,
    last_slide: 1,
    status: 'pending',
    completed_at: null,
    last_seen_notice_version: 0,
    ...overrides
  });
}

export const reportSchema = z
  .object({
    schema_version: z.number().int().positive(),
    session_id: z.string().min(1),
    classification: z.string().min(1),
    confidence: z.enum(['low', 'medium', 'high']),
    evidence_keys: z.array(z.string())
  })
  .strict();

export const sessionSchema = z
  .object({
    session_id: z.string().min(1),
    kind: z.enum(['passive', 'guided', 'imported']),
    status: z.enum([
      'active',
      'completed',
      'cancelled',
      'incomplete',
      'imported'
    ]),
    started_at: z.string().min(1),
    ended_at: z.string().nullable()
  })
  .strict();

export const preferencesSchema = z
  .object({
    language: z.enum(['system', 'es', 'en']),
    theme: z.enum(['system', 'light', 'dark']),
    motion: z.enum(['system', 'reduced', 'full']),
    glass: z.enum(['system', 'full', 'reduced', 'off'])
  })
  .strict();

export function report(
  overrides: Partial<z.infer<typeof reportSchema>> = {}
): z.infer<typeof reportSchema> {
  return reportSchema.parse({
    schema_version: 1,
    session_id: 'fixture-session',
    classification: 'indeterminate',
    confidence: 'low',
    evidence_keys: [],
    ...overrides
  });
}

export function session(
  overrides: Partial<z.infer<typeof sessionSchema>> = {}
): z.infer<typeof sessionSchema> {
  return sessionSchema.parse({
    session_id: 'fixture-session',
    kind: 'passive',
    status: 'active',
    started_at: '2026-09-19T10:00:00Z',
    ended_at: null,
    ...overrides
  });
}

export function preferences(
  overrides: Partial<z.infer<typeof preferencesSchema>> = {}
): z.infer<typeof preferencesSchema> {
  return preferencesSchema.parse({
    language: 'system',
    theme: 'system',
    motion: 'system',
    glass: 'system',
    ...overrides
  });
}

export type FakeBridgeEvent =
  keyof typeof import('../lib/bridge/schemas').eventSchemas;

export class FakeBridge implements BridgeTransport {
  private readonly listeners = new Map<string, Set<(value: unknown) => void>>();
  private snapshotValue: LiveSnapshot;
  private coverageValue: CoverageMatrix;
  private onboardingStateValue: OnboardingState;

  public constructor(
    snapshotValue = liveSnapshot(),
    coverageValue = coverage(),
    onboardingStateValue = onboardingState()
  ) {
    this.snapshotValue = snapshotValue;
    this.coverageValue = coverageValue;
    this.onboardingStateValue = onboardingStateValue;
  }

  public getLiveSnapshot(): LiveSnapshot {
    return liveSnapshot(this.snapshotValue);
  }

  public getCoverage(): CoverageMatrix {
    return coverage(this.coverageValue);
  }

  public subscribe(
    event: string,
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

  public invoke(
    command: string,
    args?: Record<string, unknown>
  ): Promise<unknown> {
    return Promise.resolve().then(() => {
      switch (command) {
        case 'get_live_snapshot':
          return this.getLiveSnapshot();
        case 'get_coverage':
        case 'recheck_coverage':
        case 'disable_advanced_access':
          return this.getCoverage();
        case 'get_onboarding_state':
          return onboardingState(this.onboardingStateValue);
        case 'set_onboarding_state': {
          const next = onboardingStateSchema.parse(args);
          this.onboardingStateValue = next;
          return onboardingState(next);
        }
        case 'request_low_level_access': {
          const request = args?.request;
          if (
            typeof request !== 'object' ||
            request === null ||
            !('action' in request) ||
            !['install', 'upgrade', 'repair'].includes(String(request.action))
          ) {
            throw new Error('Invalid access request');
          }
          return {
            state: 'install_requested',
            action: request.action,
            reboot_may_be_required: false
          };
        }
        default:
          throw new Error(`Unknown fake command: ${command}`);
      }
    });
  }

  public listen(
    event: string,
    handler: (payload: unknown) => void
  ): Promise<() => void> {
    if (
      ![
        'telemetry:snapshot',
        'collector:state',
        'coverage:changed',
        'power:context'
      ].includes(event)
    ) {
      throw new Error(`Unknown fake event: ${event}`);
    }
    return Promise.resolve(this.subscribe(event, handler));
  }
}
