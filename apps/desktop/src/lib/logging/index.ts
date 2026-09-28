import log from 'loglevel';
import { z } from 'zod';
import { invokeValidated } from '../bridge';
import { readPublicEnv, type PublicEnv } from '../config/env';

export const LOG_LEVELS = ['trace', 'debug', 'info', 'warn', 'error'] as const;
export type LogLevel = (typeof LOG_LEVELS)[number];

/** Stable event code (constitution XVII): `^[A-Z0-9_]+$`, checked again at run time. */
export type LogCode = Uppercase<string>;
export type LogFieldValue = string | number | boolean | null;
export type LogFields = Readonly<Record<string, LogFieldValue>>;

const CODE_PATTERN = /^[A-Z0-9_]+$/;
const MAX_MSG = 256;
const MAX_TARGET = 128;
const MAX_SAFE_STRING = 128;
export const REDACTED = '[redacted]';

// The same allow-list as the backend's (`src-tauri/src/logging/mod.rs`, `is_safe_field`): field
// names that only ever carry categorical or numeric diagnostics. Anything else is replaced, never
// the code or the message, which are built without personal data in the first place.
const SAFE_FIELDS: ReadonlySet<string> = new Set([
  'attempt',
  'count',
  'duration_ms',
  'dropped',
  'phase',
  'reason',
  'sequence',
  'state',
  'status',
  'size_bytes',
  'tier'
]);

const frontendLogEventSchema = z.object({
  level: z.enum(LOG_LEVELS),
  code: z.string().regex(CODE_PATTERN),
  target: z.string().min(1).max(MAX_TARGET),
  msg: z.string().min(1).max(MAX_MSG),
  fields: z.record(z.string(), z.unknown()).optional()
});

export type FrontendLogEvent = z.infer<typeof frontendLogEventSchema>;
type Clock = () => number;
type Send = (events: readonly FrontendLogEvent[]) => Promise<void>;

const windowMs = 60_000;
const maxEvents = 60;

export interface Logger {
  trace(code: LogCode, msg: string, fields?: LogFields): void;
  debug(code: LogCode, msg: string, fields?: LogFields): void;
  info(code: LogCode, msg: string, fields?: LogFields): void;
  warn(code: LogCode, msg: string, fields?: LogFields): void;
  error(code: LogCode, msg: string, fields?: LogFields): void;
}

export interface LoggerHandle {
  readonly logger: Logger;
  setDetailed(value: boolean): void;
  level(): LogLevel;
}

export interface FrontendLoggerOptions {
  readonly detailed?: boolean;
  /** A development build (`import.meta.env.DEV`); production when omitted. */
  readonly development?: boolean;
  /** `PUBLIC_LOG_LEVEL`, only honoured in a development build. */
  readonly publicLevel?: PublicEnv['PUBLIC_LOG_LEVEL'];
  readonly now?: Clock;
  readonly send?: Send;
}

const verbosity = (level: LogLevel): number => LOG_LEVELS.indexOf(level);

/**
 * Constitution XVII: development follows `PUBLIC_LOG_LEVEL` (`debug` by default); production
 * forwards only `warn`/`error`, and `debug` while «Registro detallado» lasts. `trace` never
 * reaches production.
 */
export function effectiveFrontendLevel(options: {
  readonly development: boolean;
  readonly publicLevel: PublicEnv['PUBLIC_LOG_LEVEL'];
  readonly detailed: boolean;
}): LogLevel {
  if (options.development) {
    return options.detailed &&
      verbosity('debug') < verbosity(options.publicLevel)
      ? 'debug'
      : options.publicLevel;
  }
  return options.detailed ? 'debug' : 'warn';
}

export function redactFields(
  fields: LogFields | undefined
): Record<string, LogFieldValue> | undefined {
  if (fields === undefined) return undefined;
  const redacted: Record<string, LogFieldValue> = {};
  for (const [key, value] of Object.entries(fields)) {
    const safeValue =
      typeof value !== 'string' || value.length <= MAX_SAFE_STRING;
    redacted[key] = SAFE_FIELDS.has(key) && safeValue ? value : REDACTED;
  }
  return redacted;
}

const truncate = (value: string, max: number): string =>
  value.length <= max ? value : `${value.slice(0, max - 1)}…`;

export function createLogger(
  target: string,
  options: FrontendLoggerOptions = {}
): LoggerHandle {
  const development = options.development ?? false;
  const publicLevel = options.publicLevel ?? 'debug';
  let detailed = options.detailed ?? false;
  const now = options.now ?? Date.now;
  const send = options.send ?? sendToBackend;
  const timestamps: number[] = [];
  const sink = log.getLogger(`throttlewatch:${target}`);
  const safeTarget = truncate(target || 'ui', MAX_TARGET);
  let current = effectiveFrontendLevel({ development, publicLevel, detailed });

  const applyLevel = (): void => {
    current = effectiveFrontendLevel({ development, publicLevel, detailed });
    // `false`: the level is an application preference, never loglevel's own localStorage key.
    sink.setLevel(current, false);
  };
  applyLevel();

  const emit = (
    level: LogLevel,
    code: string,
    msg: string,
    fields?: LogFields
  ): void => {
    // An event without a valid code is never dropped silently: it is kept, as an error, under a
    // code that names the defect (constitution XVII, same rule as the backend).
    const valid = CODE_PATTERN.test(code);
    const effectiveLevel: LogLevel = valid ? level : 'error';
    if (valid && verbosity(level) < verbosity(current)) return;
    const event = frontendLogEventSchema.parse({
      level: effectiveLevel,
      code: valid ? code : 'LOG_EVENT_WITHOUT_CODE',
      target: safeTarget,
      msg: truncate(valid ? msg || code : `${level} ${msg}`, MAX_MSG),
      fields: redactFields(fields)
    });
    sink[event.level](`${event.code} ${event.msg}`);
    const timestamp = now();
    while (
      timestamps[0] !== undefined &&
      timestamp - timestamps[0] >= windowMs
    ) {
      timestamps.shift();
    }
    if (timestamps.length >= maxEvents) return;
    timestamps.push(timestamp);
    void send([event]);
  };

  return {
    logger: {
      trace: (code, msg, fields) => {
        emit('trace', code, msg, fields);
      },
      debug: (code, msg, fields) => {
        emit('debug', code, msg, fields);
      },
      info: (code, msg, fields) => {
        emit('info', code, msg, fields);
      },
      warn: (code, msg, fields) => {
        emit('warn', code, msg, fields);
      },
      error: (code, msg, fields) => {
        emit('error', code, msg, fields);
      }
    },
    setDetailed(value: boolean) {
      detailed = value;
      applyLevel();
    },
    level: () => current
  };
}

export function isDetailedLoggingActive(
  value: unknown,
  now: number = Date.now()
): boolean {
  return typeof value === 'string' && Date.parse(value) > now;
}

const applicationLogger = createLogger('application', {
  development: import.meta.env.DEV,
  publicLevel: readPublicEnv().PUBLIC_LOG_LEVEL
});

export function setApplicationDetailedLogging(enabled: boolean): void {
  applicationLogger.setDetailed(enabled);
}

export function getApplicationLogger(): Logger {
  return applicationLogger.logger;
}

function describeThrown(value: unknown): string {
  if (value instanceof Error) return `${value.name}: ${value.message}`;
  if (typeof value === 'string' && value.length > 0) return value;
  return 'a non-Error value was thrown';
}

/**
 * Constitution XVII: an uncaught error or unhandled rejection in the interface is logged before
 * anything else happens to it. Returns the function that removes both listeners.
 */
export function installGlobalErrorLogging(
  target: Pick<Window, 'addEventListener' | 'removeEventListener'>,
  logger: Logger = getApplicationLogger()
): () => void {
  const onError = (event: ErrorEvent): void => {
    logger.error(
      'UI_UNHANDLED_ERROR',
      describeThrown(event.error ?? event.message),
      { state: 'error' }
    );
  };
  const onRejection = (event: PromiseRejectionEvent): void => {
    logger.error('UI_UNHANDLED_ERROR', describeThrown(event.reason), {
      state: 'unhandledrejection'
    });
  };
  target.addEventListener('error', onError);
  target.addEventListener('unhandledrejection', onRejection);
  return () => {
    target.removeEventListener('error', onError);
    target.removeEventListener('unhandledrejection', onRejection);
  };
}

async function sendToBackend(
  events: readonly FrontendLogEvent[]
): Promise<void> {
  await invokeValidated('log_frontend', { events }, z.void().or(z.null()));
}
