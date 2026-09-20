import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { z } from 'zod';
import { commandArgsSchemas, eventSchemas, parseIpcEnvelope } from './schemas';

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
    case 'set_window_state':
      return commandArgsSchemas.set_window_state;
    case 'request_low_level_access':
      return commandArgsSchemas.request_low_level_access;
    case 'disable_advanced_access':
      return commandArgsSchemas.disable_advanced_access;
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
  transport: BridgeTransport | undefined = hasTauriRuntime()
    ? tauriTransport
    : undefined
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
  transport: BridgeTransport | undefined = hasTauriRuntime()
    ? tauriTransport
    : undefined
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
