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

const emptyPayloadSchema = z.object({}).strict();

const helloPayloadSchema = z
  .object({
    app_version: z.string().regex(/^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$/),
    supported_protocols: z.array(z.number().int().positive()).min(1)
  })
  .strict();

const capabilitiesPayloadSchema = z.looseObject({
  cpu: z.looseObject({}),
  groups: z.array(z.unknown()),
  sensors: z.array(z.unknown())
});

const sampleValueSchema = z
  .object({
    sensor_id: z.string().min(1),
    number: z.number().optional(),
    boolean: z.boolean().optional(),
    status: z.enum(['ok', 'missing', 'stale', 'invalid', 'unsupported'])
  })
  .strict()
  .superRefine((value, context) => {
    const hasNumber = value.number !== undefined;
    const hasBoolean = value.boolean !== undefined;
    if (value.status === 'ok' && hasNumber === hasBoolean) {
      context.addIssue({
        code: 'custom',
        message: 'An ok sensor value must contain exactly one value.'
      });
    }
    if (value.status !== 'ok' && (hasNumber || hasBoolean)) {
      context.addIssue({
        code: 'custom',
        message: 'A non-ok sensor value cannot contain a value.'
      });
    }
  });

const samplePayloadSchema = z
  .object({
    monotonic_ms: z.number().int().nonnegative(),
    duration_ms: z.number().int().min(0).max(10000),
    values: z.array(sampleValueSchema)
  })
  .strict();

const errorPayloadSchema = z
  .object({
    code: z.string().regex(/^[A-Z0-9_]+$/),
    severity: z.enum(['info', 'recoverable', 'fatal']),
    message_key: z.string().min(1).max(256),
    context: z.record(z.string(), z.unknown()).optional()
  })
  .strict();

const payloadSchemas: Record<
  (typeof envelopeTypes)[number],
  z.ZodType<Record<string, unknown>>
> = {
  hello: helloPayloadSchema,
  hello_ack: z.record(z.string(), z.unknown()),
  capabilities: capabilitiesPayloadSchema,
  start: z.record(z.string(), z.unknown()),
  started: z.record(z.string(), z.unknown()),
  set_rate: z.record(z.string(), z.unknown()),
  snapshot: emptyPayloadSchema,
  sample: samplePayloadSchema,
  stop: emptyPayloadSchema,
  stopped: emptyPayloadSchema,
  shutdown: emptyPayloadSchema,
  error: errorPayloadSchema
};

export const ipcEnvelopeSchema = z
  .object({
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
  })
  .strict()
  .superRefine((value, context) => {
    const result = payloadSchemas[value.type].safeParse(value.payload);
    if (!result.success) {
      context.addIssue({
        code: 'custom',
        path: ['payload'],
        message: 'Payload does not match the envelope type.'
      });
    }
  });

export const structuredErrorSchema = z.object({
  code: z.string().regex(/^[A-Z0-9_]+$/),
  message_key: z.string().min(1).max(256),
  context: z.record(z.string(), z.unknown()).optional()
});

export const frontendLogEventSchema = z
  .object({
    level: z.enum(['trace', 'debug', 'info', 'warn', 'error']),
    code: z
      .string()
      .regex(/^[A-Z0-9_]+$/)
      .max(128),
    target: z.string().min(1).max(128),
    msg: z.string().min(1).max(256),
    fields: z.record(z.string(), z.unknown()).optional()
  })
  .strict();

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
  classification: z
    .enum([
      'normal',
      'hot_unproven',
      'thermal_probable',
      'thermal_confirmed',
      'power_limited',
      'platform_limited',
      'mixed_limit',
      'indeterminate'
    ])
    .nullable(),
  severity: z.enum(['boost', 'below_base']).nullable()
});

export const coverageSnapshotSchema = z.object({
  tier: z.enum(['A', 'B', 'C']),
  confidence_ceiling: z.enum(['low', 'medium', 'high']),
  advanced_access: z.enum([
    'not_needed',
    'available',
    'installable',
    'upgradable',
    'denied',
    'error'
  ])
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
  advanced_access: z.enum([
    'not_needed',
    'available',
    'installable',
    'upgradable',
    'denied',
    'error'
  ]),
  rows: z.array(coverageRowSchema),
  conclusion_key: z.string().min(1),
  from_tier: z.enum(['A', 'B', 'C']).optional(),
  to_tier: z.enum(['A', 'B', 'C']).optional(),
  reason: z.string().min(1).optional()
});

export const onboardingStateSchema = z
  .object({
    flow_version: z.number().int().positive(),
    last_slide: z.number().int().min(1).max(5),
    status: z.enum(['pending', 'completed', 'skipped']),
    completed_at: z
      .string()
      .pipe(z.iso.datetime({ offset: true }))
      .nullable(),
    last_seen_notice_version: z.number().int().nonnegative()
  })
  .strict()
  .superRefine((value, context) => {
    if (value.status === 'pending' && value.completed_at !== null) {
      context.addIssue({
        code: 'custom',
        message: 'Pending onboarding cannot be completed.'
      });
    }
    if (value.status !== 'pending' && value.completed_at === null) {
      context.addIssue({
        code: 'custom',
        message: 'Resolved onboarding needs a completion timestamp.'
      });
    }
  });

export const windowStateSchema = z
  .object({
    restored_x: z.number().int(),
    restored_y: z.number().int(),
    restored_width: z.number().int().min(480),
    restored_height: z.number().int().min(500),
    maximized: z.boolean(),
    display_fingerprint: z.string().nullable(),
    updated_at: z.string().pipe(z.iso.datetime({ offset: true }))
  })
  .strict();

export const preferencesSnapshotSchema = z
  .object({
    schema_version: z.number().int().positive(),
    values: z.record(z.string(), z.unknown()),
    adjusted: z.array(z.string())
  })
  .strict();

const setPreferenceRequestSchema = z
  .object({
    key: z.string().min(1),
    value: z.unknown(),
    expected_schema_version: z.number().int().positive()
  })
  .strict();

export const liveSnapshotSchema = telemetrySnapshotSchema.extend({
  cpu_label: z.string(),
  topology_label: z.string(),
  power_label: z.string(),
  collector_state: z.enum([
    'starting',
    'running',
    'degraded',
    'restarting',
    'stopped',
    'failed'
  ]),
  coverage: coverageSnapshotSchema,
  confidence_label: z.string(),
  active_cores: z.number().int().nonnegative().nullable(),
  platform_kind: z
    .enum(['chassis_thermal', 'external_prochot'])
    .nullable()
    .optional(),
  in_turbo_window: z.boolean(),
  cooling_potential: z
    .object({
      low_percent: z.number().int().min(0).max(100),
      high_percent: z.number().int().min(0).max(100),
      method: z.literal('power_headroom')
    })
    .nullable()
    .optional(),
  guided_result: z
    .object({
      initial_ops_per_second: z.number().nonnegative(),
      sustained_ops_per_second: z.number().nonnegative(),
      relative_percent: z.number().nullable(),
      method: z.literal('external_observation')
    })
    .nullable()
    .optional()
});

export const guidedPreflightSchema = z.object({
  sensors: z.boolean(),
  ac_power: z.boolean(),
  profile: z.boolean(),
  disk_space: z.boolean(),
  generator: z.boolean(),
  require_ac: z.boolean()
});

export const guidedPhaseSchema = z.object({
  phase: z.enum([
    'preflight',
    'ready',
    'rest',
    'warming',
    'steady_load',
    'recovery',
    'cancelling',
    'cancelled',
    'safety_stop',
    'sensor_lost',
    'error',
    'result'
  ]),
  elapsed_ms: z.number().int().nonnegative(),
  remaining_ms: z.number().int().nonnegative().nullable(),
  reason_key: z.string().nullable(),
  temperature_c: z.number().nullable(),
  thermal_limit_c: z.number().nullable(),
  active_clock_mhz: z.number().nonnegative().nullable(),
  base_clock_mhz: z.number().nonnegative().nullable(),
  throughput_ops_s: z.number().nonnegative().nullable(),
  progress_percent: z.number().min(0).max(100).nullable()
});

export const guidedFinishedEventSchema = z.object({
  session_id: z.string().min(1)
});

const analysisPointSchema = z.object({
  start_ms: z.number().int().nonnegative(),
  end_ms: z.number().int().nonnegative(),
  first: z.number().nullable(),
  last: z.number().nullable(),
  min: z.number().nullable(),
  max: z.number().nullable(),
  average: z.number().nullable(),
  quality: z.enum(['complete', 'reduced', 'missing']),
  gap: z.boolean()
});

export const analysisWindowSchema = z.object({
  session_id: z.string().min(1),
  start_ms: z.number().int().nonnegative(),
  end_ms: z.number().int().nonnegative(),
  is_aggregated: z.boolean(),
  tracks: z.array(
    z.object({ kind: z.string().min(1), points: z.array(analysisPointSchema) })
  ),
  events: z.array(
    z.object({
      id: z.string().min(1),
      kind: z.string().min(1),
      start_ms: z.number().int().nonnegative(),
      end_ms: z.number().int().nonnegative(),
      label: z.string().min(1),
      informational: z.boolean()
    })
  )
});

const exportScopeSchema = z.discriminatedUnion('kind', [
  z
    .object({ kind: z.literal('session'), session_id: z.string().min(1) })
    .strict(),
  z
    .object({ kind: z.literal('report'), session_id: z.string().min(1) })
    .strict(),
  z
    .object({
      kind: z.literal('range'),
      session_id: z.string().min(1),
      start_ms: z.number().int().nonnegative(),
      end_ms: z.number().int().positive()
    })
    .strict()
]);

const exportRequestSchema = z
  .object({
    scope: exportScopeSchema,
    format: z.enum(['csv', 'json']),
    anonymize: z.boolean()
  })
  .strict();

export const exportPreviewSchema = z
  .object({
    format: z.enum(['csv', 'json']),
    included_fields: z.array(z.string()),
    excluded_fields: z.array(z.string()),
    estimated_bytes: z.number().int().nonnegative(),
    proposed_file_name: z.string().min(1)
  })
  .strict();

export const exportResultSchema = z
  .object({
    bytes: z.number().int().nonnegative(),
    anonymized: z.boolean(),
    warnings: z.array(z.string())
  })
  .strict();

export const importResultSchema = z
  .object({
    session_id: z.string().min(1),
    schema_version: z.number().int().positive(),
    migrated: z.boolean(),
    warnings: z.array(z.string())
  })
  .strict();

export const importProgressSchema = z
  .object({
    phase: z.enum(['reading', 'validating', 'migrating', 'storing'])
  })
  .strict();

export const exportProgressSchema = z
  .object({
    done: z.number().int().nonnegative(),
    total: z.number().int().positive().optional()
  })
  .strict();

export const cpuTopologySchema = z.object({
  cores: z.array(
    z.object({
      id: z.string().min(1),
      index: z.number().int().nonnegative(),
      group: z.enum(['p', 'e', 'lp', 'ungrouped']),
      temperature_c: z.number().nullable(),
      clock_mhz: z.number().nonnegative().nullable(),
      load_percent: z.number().min(0).max(100).nullable(),
      throttling: z.boolean().nullable()
    })
  )
});

export const collectorStateEventSchema = z.object({
  state: z.enum([
    'starting',
    'running',
    'degraded',
    'restarting',
    'stopped',
    'failed'
  ]),
  attempt: z.number().int().nonnegative().optional(),
  message_key: z.string().optional()
});

export const powerContextEventSchema = z.object({
  source: z.enum(['ac', 'battery', 'unknown']),
  scheme_hash: z.string().nullable(),
  battery_percent: z.number().int().min(0).max(100).nullable().optional()
});

export const trayStateSchema = z.object({
  icon: z.enum(['normal', 'warning', 'critical', 'unknown', 'disconnected']),
  paused: z.boolean()
});

export const sessionSummarySchema = z
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
    ended_at: z.string().nullable(),
    duration_ms: z.number().int().nonnegative().nullable(),
    coverage_tier: z.enum(['A', 'B', 'C']),
    is_reference: z.boolean(),
    frame_count: z.number().int().nonnegative(),
    report_classification: z.string().nullable()
  })
  .strict();

export const sessionPageSchema = z
  .object({
    sessions: z.array(sessionSummarySchema),
    next_cursor: z.string().nullable()
  })
  .strict();

export const sessionDetailSchema = z
  .object({
    summary: sessionSummarySchema,
    report: z.record(z.string(), z.unknown()).nullable()
  })
  .strict();

export const diagnosticReportSchema = z
  .object({
    session_id: z.string().min(1),
    schema_version: z.number().int().positive(),
    report: z.record(z.string(), z.unknown()),
    frozen_at: z.string().nullable()
  })
  .strict();

export const sessionChangedEventSchema = z
  .object({
    session_id: z.string().min(1),
    status: z.string().min(1)
  })
  .strict();

export const reportFrozenEventSchema = z
  .object({ session_id: z.string().min(1) })
  .strict();

export const updateStateSchema = z.object({
  state: z.enum([
    'idle',
    'checking',
    'up_to_date',
    'available',
    'downloading',
    'verified',
    'installing',
    'error'
  ]),
  enabled: z.boolean(),
  current_version: z.string().min(1),
  last_check: z.string().nullable(),
  version: z.string().nullable(),
  notes: z.string().nullable(),
  downloaded: z.number().int().nonnegative().nullable(),
  total: z.number().int().nonnegative().nullable(),
  error_code: z.string().nullable(),
  recoverable: z.boolean().nullable()
});

export const commandResponseSchemas = {
  get_live_snapshot: liveSnapshotSchema,
  get_coverage: coverageMatrixSchema,
  recheck_coverage: coverageMatrixSchema,
  get_onboarding_state: onboardingStateSchema,
  set_onboarding_state: onboardingStateSchema,
  get_window_state: windowStateSchema,
  set_window_state: windowStateSchema,
  get_preferences: preferencesSnapshotSchema,
  get_storage_usage: z
    .object({
      database_bytes: z.number().int().nonnegative(),
      logs_bytes: z.number().int().nonnegative(),
      total_bytes: z.number().int().nonnegative(),
      session_count: z.number().int().nonnegative(),
      corrupt_backup: z.string().min(1).nullable()
    })
    .strict(),
  export_corrupt_backup: z.null(),
  get_technical_summary: z.object({ text: z.string().min(1) }).strict(),
  get_third_party_notices: z
    .object({
      entries: z.array(
        z
          .object({
            id: z.string().min(1),
            name: z.string().min(1),
            version: z.string().nullable(),
            license: z.string().min(1),
            text: z.string().min(1)
          })
          .strict()
      )
    })
    .strict(),
  log_frontend: z.null().or(z.undefined()),
  delete_monitoring_data: z
    .object({ cleared: z.array(z.string()), failed: z.array(z.string()) })
    .strict(),
  reset_application: z
    .object({ cleared: z.array(z.string()), failed: z.array(z.string()) })
    .strict(),
  open_logs_folder: z.null(),
  open_external_url: z.null(),
  set_preference: preferencesSnapshotSchema,
  request_low_level_access: z.object({
    state: z.literal('install_requested'),
    action: z.enum(['install', 'upgrade', 'repair']),
    reboot_may_be_required: z.boolean()
  }),
  disable_advanced_access: coverageMatrixSchema,
  get_guided_preflight: guidedPreflightSchema,
  start_guided: guidedPhaseSchema,
  stop_guided: guidedPhaseSchema,
  get_analysis_window: analysisWindowSchema,
  preview_export: exportPreviewSchema,
  export: exportResultSchema,
  cancel_export: z.undefined(),
  import_session: importResultSchema,
  get_cpu_topology: cpuTopologySchema,
  set_tray_paused: trayStateSchema,
  list_sessions: sessionPageSchema,
  get_session: sessionDetailSchema,
  get_report: diagnosticReportSchema,
  reevaluate_report: diagnosticReportSchema,
  delete_session: z.null(),
  set_session_reference: z.null(),
  confirm_close: z.null(),
  resolve_first_close: z.null(),
  get_update_state: updateStateSchema,
  check_for_update: updateStateSchema,
  download_update: updateStateSchema,
  install_update: updateStateSchema
} as const;

const noCommandArgs = z.undefined();

export const commandArgsSchemas = {
  get_live_snapshot: noCommandArgs,
  get_coverage: noCommandArgs,
  recheck_coverage: noCommandArgs,
  get_onboarding_state: noCommandArgs,
  // Every command below takes a single Rust parameter literally named `request`: Tauri binds JS
  // `invoke` args by parameter name, so the payload must be nested under that key, not flattened.
  set_onboarding_state: z.object({ request: onboardingStateSchema }).strict(),
  get_window_state: noCommandArgs,
  get_preferences: noCommandArgs,
  get_storage_usage: noCommandArgs,
  export_corrupt_backup: noCommandArgs,
  get_technical_summary: noCommandArgs,
  get_third_party_notices: noCommandArgs,
  log_frontend: z
    .object({ events: z.array(frontendLogEventSchema).max(60) })
    .strict(),
  delete_monitoring_data: z
    .object({
      request: z.object({ confirmation_token: z.string().min(1) }).strict()
    })
    .strict(),
  reset_application: z
    .object({
      request: z.object({ confirmation_token: z.string().min(1) }).strict()
    })
    .strict(),
  open_logs_folder: noCommandArgs,
  open_external_url: z
    .object({
      request: z.object({ target: z.enum(['help', 'source']) }).strict()
    })
    .strict(),
  set_preference: z.object({ request: setPreferenceRequestSchema }).strict(),
  set_window_state: z.object({ request: windowStateSchema }).strict(),
  request_low_level_access: z
    .object({
      request: z
        .object({ action: z.enum(['install', 'upgrade', 'repair']) })
        .strict()
    })
    .strict(),
  disable_advanced_access: noCommandArgs,
  get_guided_preflight: noCommandArgs,
  start_guided: z
    .object({
      request: z
        .object({
          profile: z.enum(['short', 'standard', 'long']),
          skip_rest: z.boolean(),
          require_ac: z.boolean()
        })
        .strict()
    })
    .strict(),
  stop_guided: noCommandArgs,
  get_analysis_window: z
    .object({
      request: z
        .object({
          session_id: z.string().min(1),
          start_ms: z.number().int().nonnegative(),
          end_ms: z.number().int().nonnegative(),
          target_points_per_track: z.number().int().min(1).max(3000)
        })
        .strict()
    })
    .strict(),
  preview_export: z.object({ request: exportRequestSchema }).strict(),
  export: z.object({ request: exportRequestSchema }).strict(),
  cancel_export: noCommandArgs,
  import_session: noCommandArgs,
  get_cpu_topology: noCommandArgs,
  set_tray_paused: z
    .object({ request: z.object({ paused: z.boolean() }).strict() })
    .strict(),
  list_sessions: z
    .object({
      request: z
        .object({
          cursor: z.string().nullable(),
          limit: z.number().int().min(1).max(100)
        })
        .strict()
    })
    .strict(),
  get_session: z
    .object({ request: z.object({ session_id: z.string().min(1) }).strict() })
    .strict(),
  get_report: z
    .object({ request: z.object({ session_id: z.string().min(1) }).strict() })
    .strict(),
  reevaluate_report: z
    .object({ request: z.object({ session_id: z.string().min(1) }).strict() })
    .strict(),
  delete_session: z
    .object({
      request: z
        .object({
          session_id: z.string().min(1),
          confirmation_token: z.string().min(1)
        })
        .strict()
    })
    .strict(),
  set_session_reference: z
    .object({
      request: z
        .object({ session_id: z.string().min(1), is_reference: z.boolean() })
        .strict()
    })
    .strict(),
  confirm_close: z
    .object({ request: z.object({ stop_operation: z.boolean() }).strict() })
    .strict(),
  resolve_first_close: z
    .object({
      request: z
        .object({ action: z.enum(['exit', 'tray', 'dismiss']) })
        .strict()
    })
    .strict(),
  get_update_state: noCommandArgs,
  check_for_update: z
    .object({ request: z.object({ manual: z.boolean() }).strict() })
    .strict(),
  download_update: noCommandArgs,
  install_update: noCommandArgs
} as const;

const updateProgressSchema = z.object({
  downloaded: z.number().int().nonnegative().nullable(),
  total: z.number().int().nonnegative().nullable()
});

const updateAvailableSchema = z.object({
  version: z.string().min(1),
  notes: z.string()
});

const updateErrorSchema = z.object({
  message_key: z.string().min(1),
  code: z.string().min(1),
  recoverable: z.boolean()
});

const closeBlockedEventSchema = z.object({
  reason: z.enum(['guided', 'export', 'download', 'install'])
});

export const eventSchemas = {
  'lifecycle:close-blocked': closeBlockedEventSchema,
  'lifecycle:close-decision-required': z.null(),
  'storage:degraded': z.null(),
  'storage:recovered': z.null(),
  'update:state-changed': updateStateSchema,
  'update:progress': updateProgressSchema,
  'update:available': updateAvailableSchema,
  'update:error': updateErrorSchema,
  'telemetry:snapshot': liveSnapshotSchema,
  'collector:state': collectorStateEventSchema,
  'coverage:changed': coverageMatrixSchema,
  'power:context': powerContextEventSchema,
  'guided:phase': guidedPhaseSchema,
  'guided:finished': guidedFinishedEventSchema,
  'tray:state': trayStateSchema,
  'session:changed': sessionChangedEventSchema,
  'report:frozen': reportFrozenEventSchema,
  'import:progress': importProgressSchema,
  'export:progress': exportProgressSchema
} as const;

export type UpdateState = z.infer<typeof updateStateSchema>;
export type TelemetrySnapshot = z.infer<typeof telemetrySnapshotSchema>;
export type CoverageSnapshot = z.infer<typeof coverageSnapshotSchema>;
export type CoverageMatrix = z.infer<typeof coverageMatrixSchema>;
export type OnboardingState = z.infer<typeof onboardingStateSchema>;
export type WindowState = z.infer<typeof windowStateSchema>;
export type PreferencesSnapshot = z.infer<typeof preferencesSnapshotSchema>;
export type LiveSnapshot = z.infer<typeof liveSnapshotSchema>;
export type CollectorStateEvent = z.infer<typeof collectorStateEventSchema>;
export type PowerContextEvent = z.infer<typeof powerContextEventSchema>;
export type TrayState = z.infer<typeof trayStateSchema>;
export type SessionSummary = z.infer<typeof sessionSummarySchema>;
export type SessionPage = z.infer<typeof sessionPageSchema>;
export type SessionDetail = z.infer<typeof sessionDetailSchema>;
export type DiagnosticReport = z.infer<typeof diagnosticReportSchema>;
export type SessionChangedEvent = z.infer<typeof sessionChangedEventSchema>;
export type AccessRequestResult = z.infer<
  typeof commandResponseSchemas.request_low_level_access
>;
export type GuidedPreflight = z.infer<typeof guidedPreflightSchema>;
export type CpuTopology = z.infer<typeof cpuTopologySchema>;
export type GuidedFinishedEvent = z.infer<typeof guidedFinishedEventSchema>;
export type GuidedPhase = z.infer<typeof guidedPhaseSchema>;
export type AnalysisWindow = z.infer<typeof analysisWindowSchema>;
export type ExportPreview = z.infer<typeof exportPreviewSchema>;
export type ExportResult = z.infer<typeof exportResultSchema>;
export type ImportResult = z.infer<typeof importResultSchema>;
export type ImportProgress = z.infer<typeof importProgressSchema>;
export type ExportProgress = z.infer<typeof exportProgressSchema>;
export type CloseBlockedEvent = z.infer<typeof closeBlockedEventSchema>;

export function parseIpcEnvelope(value: unknown): IpcEnvelope {
  return ipcEnvelopeSchema.parse(value);
}
