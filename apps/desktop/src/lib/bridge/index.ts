import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { z } from 'zod';
import { eventSchemas, parseIpcEnvelope } from './schemas';

export type BridgeError = {
  readonly kind: 'validation' | 'backend' | 'transport';
  readonly code: string;
  readonly message_key: string;
  readonly details?: unknown;
};

export type BridgeResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: BridgeError };

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
  responseSchema: z.ZodType<T>
): Promise<BridgeResult<T>> {
  try {
    const raw = await invoke<unknown>(command, args);
    return { ok: true, value: responseSchema.parse(raw) };
  } catch (error) {
    const kind = error instanceof z.ZodError ? 'validation' : 'backend';
    return { ok: false, error: errorFromUnknown(kind, error) };
  }
}

export function parseEvent(event: 'telemetry:snapshot', value: unknown): BridgeResult<import('./schemas').LiveSnapshot>;
export function parseEvent(event: 'collector:state', value: unknown): BridgeResult<import('./schemas').CollectorStateEvent>;
export function parseEvent(event: 'coverage:changed', value: unknown): BridgeResult<import('./schemas').CoverageMatrix>;
export function parseEvent(event: 'power:context', value: unknown): BridgeResult<import('./schemas').PowerContextEvent>;
export function parseEvent(event: keyof typeof eventSchemas, value: unknown): BridgeResult<unknown> {
  try {
    return { ok: true, value: eventSchemas[event].parse(value) };
  } catch (error) {
    return { ok: false, error: errorFromUnknown('validation', error) };
  }
}

export async function listenValidated(
  event: keyof typeof eventSchemas,
  handler: (value: unknown) => void
): Promise<UnlistenFn> {
  return listen<unknown>(event, (message) => {
    const parsed = eventSchemas[event].safeParse(message.payload);
    if (parsed.success) handler(parsed.data);
  });
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
