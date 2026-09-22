import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { z } from 'zod';
import { commandArgsSchemas, eventSchemas, parseIpcEnvelope } from './schemas';

declare global {
  interface Window {
    __THROTTLEWATCH_FAKE_BRIDGE__?: BridgeTransport;
  }
}

export interface BridgeTransport {
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
  listen(
    event: string,
    handler: (payload: unknown) => void
  ): Promise<UnlistenFn>;
}

export type BridgeError = {
  readonly kind: 'validation' | 'backend' | 'transport';
  readonly code: string;
  readonly message_key: string;
  readonly details?: unknown;
};

export type BridgeResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: BridgeError };

const TAURI_UNAVAILABLE_ERROR: BridgeError = {
  kind: 'transport',
  code: 'TAURI_UNAVAILABLE',
  message_key: 'bridge.tauri_unavailable'
};

function hasTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  return '__TAURI_INTERNALS__' in window;
}

/**
 * The fake transport that Playwright injects exists only in the `e2e` build mode (and in Vitest).
 * The build mode is a constant that Vite writes into the bundle, so in a release build the check
 * is always false and a script running inside the WebView cannot replace the bridge and forge
 * backend answers (the property name still appears in the bundle; it is never read).
 */
export function fakeBridgeAllowed(mode: string): boolean {
  return mode === 'e2e' || mode === 'test';
}

function defaultTransport(): BridgeTransport | undefined {
  if (
    fakeBridgeAllowed(import.meta.env.MODE) &&
    typeof window !== 'undefined' &&
    window.__THROTTLEWATCH_FAKE_BRIDGE__
  ) {
    return window.__THROTTLEWATCH_FAKE_BRIDGE__;
  }
  return hasTauriRuntime() ? tauriTransport : undefined;
}

const tauriTransport: BridgeTransport = {
  invoke: (command, args) => invoke<unknown>(command, args),
  listen: (event, handler) =>
    listen<unknown>(event, (message) => {
      handler(message.payload);
    })
};

function argsSchemaFor(command: string): z.ZodType | undefined {
  switch (command) {
    case 'get_live_snapshot':
      return commandArgsSchemas.get_live_snapshot;
    case 'get_coverage':
      return commandArgsSchemas.get_coverage;
    case 'recheck_coverage':
      return commandArgsSchemas.recheck_coverage;
    case 'get_onboarding_state':
      return commandArgsSchemas.get_onboarding_state;
    case 'set_onboarding_state':
      return commandArgsSchemas.set_onboarding_state;
    case 'get_window_state':
      return commandArgsSchemas.get_window_state;
    case 'get_preferences':
      return commandArgsSchemas.get_preferences;
    case 'get_storage_usage':
      return commandArgsSchemas.get_storage_usage;
    case 'export_corrupt_backup':
      return commandArgsSchemas.export_corrupt_backup;
    case 'get_technical_summary':
      return commandArgsSchemas.get_technical_summary;
    case 'get_third_party_notices':
      return commandArgsSchemas.get_third_party_notices;
    case 'log_frontend':
      return commandArgsSchemas.log_frontend;
    case 'delete_monitoring_data':
      return commandArgsSchemas.delete_monitoring_data;
    case 'reset_application':
      return commandArgsSchemas.reset_application;
    case 'open_logs_folder':
      return commandArgsSchemas.open_logs_folder;
    case 'open_external_url':
      return commandArgsSchemas.open_external_url;
    case 'set_preference':
      return commandArgsSchemas.set_preference;
    case 'set_window_state':
      return commandArgsSchemas.set_window_state;
    case 'request_low_level_access':
      return commandArgsSchemas.request_low_level_access;
    case 'disable_advanced_access':
      return commandArgsSchemas.disable_advanced_access;
    case 'get_guided_preflight':
      return commandArgsSchemas.get_guided_preflight;
    case 'start_guided':
      return commandArgsSchemas.start_guided;
    case 'stop_guided':
      return commandArgsSchemas.stop_guided;
    case 'get_analysis_window':
      return commandArgsSchemas.get_analysis_window;
    case 'preview_export':
      return commandArgsSchemas.preview_export;
    case 'export':
      return commandArgsSchemas.export;
    case 'cancel_export':
      return commandArgsSchemas.cancel_export;
    case 'import_session':
      return commandArgsSchemas.import_session;
    case 'get_cpu_topology':
      return commandArgsSchemas.get_cpu_topology;
    case 'set_tray_paused':
      return commandArgsSchemas.set_tray_paused;
    case 'list_sessions':
      return commandArgsSchemas.list_sessions;
    case 'get_session':
      return commandArgsSchemas.get_session;
    case 'get_report':
      return commandArgsSchemas.get_report;
    case 'delete_session':
      return commandArgsSchemas.delete_session;
    case 'set_session_reference':
      return commandArgsSchemas.set_session_reference;
    case 'confirm_close':
      return commandArgsSchemas.confirm_close;
    case 'resolve_first_close':
      return commandArgsSchemas.resolve_first_close;
    case 'get_update_state':
      return commandArgsSchemas.get_update_state;
    case 'reevaluate_report':
      return commandArgsSchemas.reevaluate_report;
    case 'check_for_update':
      return commandArgsSchemas.check_for_update;
    case 'download_update':
      return commandArgsSchemas.download_update;
    case 'install_update':
      return commandArgsSchemas.install_update;
    default:
      return undefined;
  }
}

function errorFromUnknown(
  kind: BridgeError['kind'],
  error: unknown
): BridgeError {
  if (error instanceof z.ZodError) {
    return {
      kind,
      code: 'BRIDGE_VALIDATION_FAILED',
      message_key: 'bridge.validation_failed',
      details: error.issues
    };
  }
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message_key' in error
  ) {
    const candidate = error;
    if (
      typeof candidate.code === 'string' &&
      typeof candidate.message_key === 'string'
    ) {
      return {
        kind,
        code: candidate.code,
        message_key: candidate.message_key,
        details: error
      };
    }
  }
  return {
    kind,
    code: 'BRIDGE_UNKNOWN_ERROR',
    message_key: 'bridge.unknown_error',
    details: error
  };
}

export async function invokeValidated<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  responseSchema: z.ZodType<T>,
  transport: BridgeTransport | undefined = defaultTransport()
): Promise<BridgeResult<T>> {
  if (transport === undefined) {
    return { ok: false, error: TAURI_UNAVAILABLE_ERROR };
  }

  try {
    argsSchemaFor(command)?.parse(args);
    const raw = await transport.invoke(command, args);
    return { ok: true, value: responseSchema.parse(raw) };
  } catch (error) {
    const kind = error instanceof z.ZodError ? 'validation' : 'backend';
    return { ok: false, error: errorFromUnknown(kind, error) };
  }
}

export function parseEvent(
  event: 'telemetry:snapshot',
  value: unknown
): BridgeResult<import('./schemas').LiveSnapshot>;
export function parseEvent(
  event: 'collector:state',
  value: unknown
): BridgeResult<import('./schemas').CollectorStateEvent>;
export function parseEvent(
  event: 'coverage:changed',
  value: unknown
): BridgeResult<import('./schemas').CoverageMatrix>;
export function parseEvent(
  event: 'power:context',
  value: unknown
): BridgeResult<import('./schemas').PowerContextEvent>;
export function parseEvent(
  event: 'guided:phase',
  value: unknown
): BridgeResult<import('./schemas').GuidedPhase>;
export function parseEvent(
  event: 'tray:state',
  value: unknown
): BridgeResult<import('./schemas').TrayState>;
export function parseEvent(
  event: 'session:changed',
  value: unknown
): BridgeResult<import('./schemas').SessionChangedEvent>;
export function parseEvent(
  event: 'report:frozen',
  value: unknown
): BridgeResult<{ session_id: string }>;
export function parseEvent(
  event: 'import:progress',
  value: unknown
): BridgeResult<import('./schemas').ImportProgress>;
export function parseEvent(
  event: 'export:progress',
  value: unknown
): BridgeResult<import('./schemas').ExportProgress>;
export function parseEvent(
  event: keyof typeof eventSchemas,
  value: unknown
): BridgeResult<unknown> {
  try {
    return { ok: true, value: eventSchemas[event].parse(value) };
  } catch (error) {
    return { ok: false, error: errorFromUnknown('validation', error) };
  }
}

export async function listenValidated(
  event: keyof typeof eventSchemas,
  handler: (value: unknown) => void,
  transport: BridgeTransport | undefined = defaultTransport()
): Promise<UnlistenFn> {
  if (transport === undefined) {
    return () => undefined;
  }

  return transport.listen(event, (value) => {
    const parsed = eventSchemas[event].safeParse(value);
    if (parsed.success) handler(parsed.data);
  });
}

export function createBridge(transport: BridgeTransport) {
  return {
    invokeValidated: <T>(
      command: string,
      args: Record<string, unknown> | undefined,
      responseSchema: z.ZodType<T>
    ) => invokeValidated(command, args, responseSchema, transport),
    listenValidated: (
      event: keyof typeof eventSchemas,
      handler: (value: unknown) => void
    ) => listenValidated(event, handler, transport)
  };
}

export function validateEnvelope(
  value: unknown
): BridgeResult<ReturnType<typeof parseIpcEnvelope>> {
  try {
    return { ok: true, value: parseIpcEnvelope(value) };
  } catch (error) {
    return { ok: false, error: errorFromUnknown('validation', error) };
  }
}
