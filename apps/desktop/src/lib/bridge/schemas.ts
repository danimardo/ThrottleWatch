import { z } from 'zod';

const envelopeTypes = [
  'hello',
  'hello_ack',
  'capabilities',
  'start',
  'started',
  'set_rate',
  'snapshot',
  'sample',
  'stop',
  'stopped',
  'shutdown',
  'error'
] as const;

export const ipcEnvelopeSchema = z.object({
  protocol_version: z.literal(1),
  session_nonce: z
    .string()
    .min(16)
    .max(128)
    .regex(/^[A-Za-z0-9_-]+$/),
  sequence: z.number().int().nonnegative(),
  timestamp_utc: z.iso.datetime({ offset: true }),
  type: z.enum(envelopeTypes),
  payload: z.record(z.string(), z.unknown())
});

export const structuredErrorSchema = z.object({
  code: z.string().regex(/^[A-Z0-9_]+$/),
  message_key: z.string().min(1).max(256),
  context: z.record(z.string(), z.unknown()).optional()
});

export type IpcEnvelope = z.infer<typeof ipcEnvelopeSchema>;
export type StructuredError = z.infer<typeof structuredErrorSchema>;

export const telemetrySnapshotSchema = z.object({
  captured_at_ms: z.number().int().nonnegative(),
  freshness: z.enum(['fresh', 'stale', 'disconnected']),
  age_ms: z.number().int().nonnegative(),
  temperature_c: z.number().nullable(),
  thermal_limit_c: z.number().nullable(),
  thermal_margin_c: z.number().nullable(),
  load_percent: z.number().min(0).max(100).nullable(),
  active_clock_mhz: z.number().nonnegative().nullable(),
  base_clock_mhz: z.number().nonnegative().nullable(),
  package_power_w: z.number().nonnegative().nullable(),
  power_limit_w: z.number().nonnegative().nullable(),
  classification: z.enum([
    'normal', 'hot_unproven', 'thermal_probable', 'thermal_confirmed',
    'power_limited', 'platform_limited', 'mixed_limit', 'indeterminate'
  ]).nullable(),
  severity: z.enum(['boost', 'below_base']).nullable()
});

export const coverageSnapshotSchema = z.object({
  tier: z.enum(['A', 'B', 'C']),
  confidence_ceiling: z.enum(['low', 'medium', 'high']),
  advanced_access: z.enum(['not_needed', 'available', 'installable', 'denied', 'error'])
});

const coverageRowSchema = z.object({
  id: z.string().min(1),
  label: z.string().min(1),
  available: z.boolean(),
  quality: z.enum(['direct', 'derived', 'substitute', 'unknown']).optional(),
  quality_label: z.string().optional(),
  source_label: z.string().optional(),
  reason_key: z.string().optional()
});

export const coverageMatrixSchema = z.object({
  tier: z.enum(['A', 'B', 'C']),
  confidence_ceiling: z.enum(['low', 'medium', 'high']),
  advanced_access: z.enum(['not_needed', 'available', 'installable', 'denied', 'error']),
  rows: z.array(coverageRowSchema),
  conclusion_key: z.string().min(1)
});

export const liveSnapshotSchema = telemetrySnapshotSchema.extend({
  cpu_label: z.string(),
  topology_label: z.string(),
  power_label: z.string(),
  collector_state: z.enum(['starting', 'running', 'degraded', 'restarting', 'stopped', 'failed']),
  coverage: coverageSnapshotSchema,
  confidence_label: z.string(),
  active_cores: z.number().int().nonnegative().nullable(),
  platform_kind: z.enum(['chassis_thermal', 'external_prochot']).nullable().optional(),
  in_turbo_window: z.boolean(),
  cooling_potential: z.object({
    low_percent: z.number().int().min(0).max(100),
    high_percent: z.number().int().min(0).max(100),
    method: z.literal('power_headroom')
  }).nullable().optional(),
  guided_result: z.object({
    initial_ops_per_second: z.number().nonnegative(),
    sustained_ops_per_second: z.number().nonnegative(),
    relative_percent: z.number().nullable(),
    method: z.literal('external_observation')
  }).nullable().optional()
});

export const collectorStateEventSchema = z.object({
  state: z.enum(['starting', 'running', 'degraded', 'restarting', 'stopped', 'failed']),
  attempt: z.number().int().nonnegative().optional(),
  message_key: z.string().optional()
});

export const powerContextEventSchema = z.object({
  source: z.enum(['ac', 'battery', 'unknown']),
  scheme_hash: z.string().nullable(),
  battery_percent: z.number().int().min(0).max(100).nullable().optional()
});

export const commandResponseSchemas = {
  get_live_snapshot: liveSnapshotSchema,
  get_coverage: coverageMatrixSchema,
  recheck_coverage: coverageMatrixSchema,
  request_low_level_access: z.object({
    state: z.literal('install_requested'),
    action: z.enum(['install', 'upgrade', 'repair']),
    reboot_may_be_required: z.boolean()
  }),
  disable_advanced_access: coverageMatrixSchema
} as const;

export const eventSchemas = {
  'telemetry:snapshot': liveSnapshotSchema,
  'collector:state': collectorStateEventSchema,
  'coverage:changed': coverageMatrixSchema,
  'power:context': powerContextEventSchema
} as const;

export type TelemetrySnapshot = z.infer<typeof telemetrySnapshotSchema>;
export type CoverageSnapshot = z.infer<typeof coverageSnapshotSchema>;
export type CoverageMatrix = z.infer<typeof coverageMatrixSchema>;
export type LiveSnapshot = z.infer<typeof liveSnapshotSchema>;
export type CollectorStateEvent = z.infer<typeof collectorStateEventSchema>;
export type PowerContextEvent = z.infer<typeof powerContextEventSchema>;
export type AccessRequestResult = z.infer<typeof commandResponseSchemas.request_low_level_access>;

export function parseIpcEnvelope(value: unknown): IpcEnvelope {
  return ipcEnvelopeSchema.parse(value);
}
