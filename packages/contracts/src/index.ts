export type ProtocolVersion = 1;

export type ExportFormat = 'csv' | 'json';
export type ExportScope =
  | { readonly kind: 'session'; readonly session_id: string }
  | { readonly kind: 'report'; readonly session_id: string }
  | {
      readonly kind: 'range';
      readonly session_id: string;
      readonly start_ms: number;
      readonly end_ms: number;
    };

export interface CoverageChange {
  readonly at: number;
  readonly from_tier: 'A' | 'B' | 'C';
  readonly to_tier: 'A' | 'B' | 'C';
  readonly reason: string;
}

/** Versioned evidence format. It is deliberately separate from the IPC envelope. */
export interface ExportDocument {
  readonly schema_version: 1;
  readonly kind: 'session';
  readonly session: {
    readonly session_id: string;
    readonly status: string;
    readonly started_at: string;
    readonly ended_at: string | null;
    readonly duration_ms: number | null;
    readonly coverage_tier: 'A' | 'B' | 'C';
    readonly cpu_vendor: string;
    readonly cpu_model: string;
    readonly topology: string;
    readonly ruleset_version: string;
  };
  readonly samples: readonly Record<string, unknown>[];
  readonly events: readonly Record<string, unknown>[];
  readonly report:
    | (Record<string, unknown> & {
        readonly coverage_history?: readonly CoverageChange[];
      })
    | null;
}
export type MessageType =
  | 'hello'
  | 'hello_ack'
  | 'capabilities'
  | 'start'
  | 'started'
  | 'set_rate'
  | 'snapshot'
  | 'sample'
  | 'stop'
  | 'stopped'
  | 'shutdown'
  | 'error';

export interface Envelope<TType extends MessageType, TPayload extends object> {
  readonly protocol_version: ProtocolVersion;
  readonly session_nonce: string;
  readonly sequence: number;
  readonly timestamp_utc: string;
  readonly type: TType;
  readonly payload: TPayload;
}

export interface HelloPayload {
  readonly app_version: string;
  readonly supported_protocols: readonly number[];
}

export interface HelloAckPayload {
  readonly agent_version: string;
  readonly selected_protocol: ProtocolVersion;
  readonly runtime: string;
  readonly low_level_access: LowLevelAccess;
}

export interface LowLevelAccess {
  readonly state:
    'available' | 'reduced' | 'missing' | 'denied' | 'error' | 'unknown';
  readonly provider?: string | null;
  readonly details_code?: string | null;
}

export interface CapabilitiesPayload {
  readonly cpu: CpuCapabilities;
  readonly groups: readonly CpuGroup[];
  readonly sensors: readonly SensorDescriptor[];
}

export interface CpuCapabilities {
  readonly vendor: 'intel' | 'amd' | 'other' | 'unknown';
  readonly display_name: string;
  readonly family?: string | null;
  readonly model?: string | null;
  readonly stepping?: string | null;
  readonly architecture?: string | null;
  readonly logical_processors: number;
  readonly physical_cores?: number | null;
  readonly hybrid: boolean;
  readonly virtualized?: boolean;
}

export interface CpuGroup {
  readonly id: string;
  readonly kind: 'p' | 'e' | 'lp_e' | 'homogeneous' | 'unknown';
  readonly physical_count?: number | null;
  readonly logical_count: number;
  readonly logical_ids?: readonly number[];
}

export interface SensorDescriptor {
  readonly id: string;
  readonly source_id: string;
  readonly source_name: string;
  readonly metric:
    | 'temperature'
    | 'thermal_headroom'
    | 'load'
    | 'active_clock'
    | 'base_clock'
    | 'clock'
    | 'power'
    | 'power_limit'
    | 'voltage'
    | 'thermal_flag'
    | 'prochot_flag'
    | 'power_flag'
    | 'current_flag';
  readonly scope: 'package' | 'group' | 'core' | 'thread' | 'system';
  readonly scope_ref?: string | null;
  readonly unit:
    'celsius' | 'percent' | 'megahertz' | 'watt' | 'volt' | 'boolean';
  readonly quality: 'direct' | 'derived' | 'substitute' | 'unknown';
  readonly metadata?: Record<string, unknown>;
}

export interface StartPayload {
  readonly interval_ms: number;
  readonly detail: 'representative' | 'per_group' | 'per_core';
}

export interface SetRatePayload {
  readonly interval_ms: number;
}

export interface SamplePayload {
  readonly monotonic_ms: number;
  readonly duration_ms: number;
  readonly values: readonly SensorValue[];
}

export type SensorValue =
  | {
      readonly sensor_id: string;
      readonly number: number;
      readonly status: 'ok';
    }
  | {
      readonly sensor_id: string;
      readonly boolean: boolean;
      readonly status: 'ok';
    }
  | {
      readonly sensor_id: string;
      readonly status: 'missing' | 'stale' | 'invalid' | 'unsupported';
    };

export interface ErrorPayload {
  readonly code: string;
  readonly severity: 'info' | 'recoverable' | 'fatal';
  readonly message_key: string;
  readonly context?: Record<string, unknown>;
}

export type HelloMessage = Envelope<'hello', HelloPayload>;
export type HelloAckMessage = Envelope<'hello_ack', HelloAckPayload>;
export type CapabilitiesMessage = Envelope<'capabilities', CapabilitiesPayload>;
export type StartMessage = Envelope<'start' | 'started', StartPayload>;
export type SetRateMessage = Envelope<'set_rate', SetRatePayload>;
export type SampleMessage = Envelope<'sample', SamplePayload>;
export type ErrorMessage = Envelope<'error', ErrorPayload>;

export type IpcMessage =
  | HelloMessage
  | HelloAckMessage
  | CapabilitiesMessage
  | StartMessage
  | SetRateMessage
  | SampleMessage
  | ErrorMessage
  | Envelope<
      'snapshot' | 'stop' | 'stopped' | 'shutdown',
      Record<string, never>
    >;
