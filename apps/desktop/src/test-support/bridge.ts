import {
  coverageMatrixSchema,
  liveSnapshotSchema,
  onboardingStateSchema,
  preferencesSnapshotSchema,
  sessionSummarySchema,
  type SessionSummary,
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

export function sessionSummary(
  overrides: Partial<SessionSummary> = {}
): SessionSummary {
  return sessionSummarySchema.parse({
    session_id: 'fixture-session',
    kind: 'guided',
    status: 'completed',
    started_at: '2026-09-19T10:00:00Z',
    ended_at: '2026-09-19T10:05:00Z',
    duration_ms: 300_000,
    coverage_tier: 'A',
    is_reference: false,
    frame_count: 300,
    report_classification: 'normal',
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
  private preferencesValue = preferencesSnapshotSchema.parse({
    schema_version: 1,
    values: {
      'locale.mode': 'system',
      'appearance.theme': 'system',
      'appearance.motion': 'system',
      'appearance.glass': 'system',
      'sampling.profile': 'normal',
      'sampling.on_battery': 'keep',
      'sampling.per_core_history': false,
      'history.retention': '7d',
      'notifications.enabled': false,
      'notifications.quiet_period': null,
      'tray.monitoring_enabled': false,
      'lifecycle.close_action': 'unset',
      'startup.enabled': false,
      'startup.mode': 'window',
      'guided.duration': 'standard',
      'guided.require_ac': false,
      'guided.notify_on_finish': false,
      'privacy.anonymize_exports': true,
      'updates.enabled': false,
      'logging.detailed_until': null
    },
    adjusted: []
  });
  private trayPaused = false;
  private storageUsage = {
    database_bytes: 4096,
    logs_bytes: 0,
    total_bytes: 4096,
    session_count: 1
  };
  private sessionsValue: SessionSummary[];
  private readonly reports = new Map<string, Record<string, unknown>>();

  public constructor(
    snapshotValue = liveSnapshot(),
    coverageValue = coverage(),
    onboardingStateValue = onboardingState(),
    sessionsValue = [sessionSummary()]
  ) {
    this.snapshotValue = snapshotValue;
    this.coverageValue = coverageValue;
    this.onboardingStateValue = onboardingStateValue;
    this.sessionsValue = sessionsValue.map((value) => sessionSummary(value));
    for (const value of this.sessionsValue) {
      if (value.report_classification !== null) {
        this.reports.set(value.session_id, {
          schema_version: 1,
          session_id: value.session_id,
          classification: value.report_classification,
          confidence:
            value.coverage_tier === 'A'
              ? 'high'
              : value.coverage_tier === 'B'
                ? 'medium'
                : 'low',
          events: []
        });
      }
    }
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
        case 'get_update_state':
        case 'check_for_update':
        case 'download_update':
        case 'install_update':
          return {
            state: 'idle',
            enabled: false,
            current_version: '0.1.0',
            last_check: null,
            version: null,
            notes: null,
            downloaded: null,
            total: null,
            error_code: null,
            recoverable: null
          };
        case 'get_live_snapshot':
          return this.getLiveSnapshot();
        case 'get_coverage':
        case 'recheck_coverage':
        case 'disable_advanced_access':
          return this.getCoverage();
        case 'get_onboarding_state':
          return onboardingState(this.onboardingStateValue);
        case 'get_preferences':
          return this.preferencesValue;
        case 'get_storage_usage':
          return { ...this.storageUsage };
        case 'get_technical_summary':
          return {
            text: 'ThrottleWatch 0.1.0\nIPC protocol: 1\nCollector: running\nStored sessions: 0\nDatabase bytes: 4096\n\nThis summary contains no CPU identifiers, paths, URLs, or raw log lines.'
          };
        case 'get_third_party_notices':
          return {
            entries: [
              {
                id: 'throttlewatch',
                name: 'ThrottleWatch',
                version: '0.1.0',
                license: 'GPL-3.0-only',
                text: 'ThrottleWatch source license.'
              }
            ]
          };
        case 'delete_monitoring_data':
          this.sessionsValue = [];
          this.reports.clear();
          this.storageUsage = { ...this.storageUsage, session_count: 0 };
          return { cleared: ['data', 'logs'], failed: [] };
        case 'reset_application':
          this.sessionsValue = [];
          this.reports.clear();
          this.storageUsage = {
            database_bytes: 4096,
            logs_bytes: 0,
            total_bytes: 4096,
            session_count: 0
          };
          return { cleared: ['data_and_preferences', 'logs'], failed: [] };
        case 'open_logs_folder':
        case 'open_external_url':
        case 'log_frontend':
          return null;
        case 'set_preference': {
          const request = args?.request;
          if (
            typeof request !== 'object' ||
            request === null ||
            !('key' in request) ||
            !('value' in request) ||
            !('expected_schema_version' in request)
          ) {
            throw new Error('Invalid preference request');
          }
          const key = String(request.key);
          this.preferencesValue = preferencesSnapshotSchema.parse({
            ...this.preferencesValue,
            values: { ...this.preferencesValue.values, [key]: request.value },
            adjusted: []
          });
          return this.preferencesValue;
        }
        case 'set_onboarding_state': {
          // Tauri binds this command's one parameter, named `request`: the payload is nested.
          const next = onboardingStateSchema.parse(args?.request);
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
        case 'set_tray_paused': {
          const request = args?.request;
          if (
            typeof request !== 'object' ||
            request === null ||
            !('paused' in request) ||
            typeof request.paused !== 'boolean'
          ) {
            throw new Error('Invalid tray request');
          }
          this.trayPaused = request.paused;
          const value = {
            icon: this.trayPaused ? 'unknown' : 'normal',
            paused: this.trayPaused
          } as const;
          this.emit('tray:state', value);
          return value;
        }
        case 'preview_export': {
          const request = args?.request;
          if (typeof request !== 'object' || request === null) {
            throw new Error('Invalid export request');
          }
          const format =
            'format' in request && request.format === 'json' ? 'json' : 'csv';
          return {
            format,
            included_fields:
              format === 'json'
                ? ['classification', 'events', 'coverage_tier']
                : ['timestamp_utc', 'monotonic_ms', 'sensor_id', 'value'],
            excluded_fields: [],
            estimated_bytes: 512,
            proposed_file_name: `throttlewatch_session_export.${format}`
          };
        }
        case 'export':
          return { bytes: 512, anonymized: true, warnings: [] };
        case 'cancel_export':
          return undefined;
        case 'confirm_close':
        case 'resolve_first_close':
          return null;
        case 'import_session': {
          const importedId = 'imported-session-1';
          this.sessionsValue.push(
            sessionSummary({
              session_id: importedId,
              kind: 'imported',
              status: 'imported'
            })
          );
          this.emit('session:changed', {
            session_id: importedId,
            status: 'imported'
          });
          return {
            session_id: importedId,
            schema_version: 1,
            migrated: false,
            warnings: []
          };
        }
        case 'list_sessions':
          return {
            sessions: this.sessionsValue.map((value) => sessionSummary(value)),
            next_cursor: null
          };
        case 'get_session': {
          const request = args?.request;
          const sessionId =
            typeof request === 'object' &&
            request !== null &&
            'session_id' in request
              ? String(request.session_id)
              : '';
          const summary = this.sessionsValue.find(
            (value) => value.session_id === sessionId
          );
          if (!summary) throw new Error('Session not found');
          return {
            summary: sessionSummary(summary),
            report: this.reports.get(sessionId) ?? null
          };
        }
        case 'get_report': {
          const request = args?.request;
          const sessionId =
            typeof request === 'object' &&
            request !== null &&
            'session_id' in request
              ? String(request.session_id)
              : '';
          const reportValue = this.reports.get(sessionId);
          if (!reportValue) throw new Error('Report not found');
          return {
            session_id: sessionId,
            schema_version: 1,
            report: reportValue,
            frozen_at: '2026-09-19T10:05:00Z'
          };
        }

        case 'reevaluate_report': {
          const request = args?.request;
          const sessionId =
            typeof request === 'object' &&
            request !== null &&
            'session_id' in request
              ? String(request.session_id)
              : '';
          const reportValue = this.reports.get(sessionId);
          if (!reportValue) throw new Error('Report not found');
          return {
            session_id: sessionId,
            schema_version: 1,
            report: reportValue,
            frozen_at: null
          };
        }
        case 'delete_session': {
          const request = args?.request;
          const sessionId =
            typeof request === 'object' &&
            request !== null &&
            'session_id' in request
              ? String(request.session_id)
              : '';
          const index = this.sessionsValue.findIndex(
            (value) => value.session_id === sessionId
          );
          if (index < 0) throw new Error('Session not found');
          if (this.sessionsValue[index]?.status === 'active')
            throw new Error('Active session cannot be deleted');
          this.sessionsValue.splice(index, 1);
          this.reports.delete(sessionId);
          this.emit('session:changed', {
            session_id: sessionId,
            status: 'deleted'
          });
          return null;
        }
        case 'set_session_reference': {
          const request = args?.request;
          const sessionId =
            typeof request === 'object' &&
            request !== null &&
            'session_id' in request
              ? String(request.session_id)
              : '';
          const isReference =
            typeof request === 'object' &&
            request !== null &&
            'is_reference' in request
              ? Boolean(request.is_reference)
              : false;
          const index = this.sessionsValue.findIndex(
            (value) => value.session_id === sessionId
          );
          const current = this.sessionsValue[index];
          if (
            index < 0 ||
            current?.kind !== 'guided' ||
            current.status !== 'completed'
          ) {
            throw new Error('Only completed guided sessions can be references');
          }
          this.sessionsValue[index] = sessionSummary({
            ...current,
            is_reference: isReference
          });
          this.emit('session:changed', {
            session_id: sessionId,
            status: isReference ? 'reference' : 'completed'
          });
          return null;
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
        'power:context',
        'tray:state',
        'session:changed',
        'report:frozen',
        'import:progress',
        'export:progress'
      ].includes(event)
    ) {
      throw new Error(`Unknown fake event: ${event}`);
    }
    return Promise.resolve(this.subscribe(event, handler));
  }
}
