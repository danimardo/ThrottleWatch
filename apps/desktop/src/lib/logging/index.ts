import log from 'loglevel';
import { z } from 'zod';
import { invokeValidated } from '../bridge';

const levels = ['trace', 'debug', 'info', 'warn', 'error'] as const;
const frontendLogEventSchema = z.object({
  level: z.enum(levels),
  code: z.string().regex(/^[A-Z0-9_]+$/),
  target: z.string().min(1).max(128),
  msg: z.string().min(1).max(256),
  fields: z.record(z.string(), z.unknown()).optional()
});

export type FrontendLogEvent = z.infer<typeof frontendLogEventSchema>;
type Clock = () => number;
type Send = (events: readonly FrontendLogEvent[]) => Promise<void>;
type LogLevel = (typeof levels)[number];

const windowMs = 60_000;
const maxEvents = 60;

export interface FrontendLoggerOptions {
  readonly detailed?: boolean;
  readonly now?: Clock;
  readonly send?: Send;
}

export function createLogger(
  target: string,
  options: FrontendLoggerOptions = {}
) {
  let detailed = options.detailed ?? false;
  const now = options.now ?? Date.now;
  const send = options.send ?? sendToBackend;
  const timestamps: number[] = [];
  const logger = log.getLogger(`throttlewatch:${target}`);
  const originalFactory = logger.methodFactory;

  logger.methodFactory = (methodName, methodLevel, loggerName) => {
    const original = originalFactory(methodName, methodLevel, loggerName);
    return (...args: unknown[]) => {
      original(...args);
      const level: LogLevel = methodName;
      if (!levels.includes(level)) {
        return;
      }
      if (!detailed && (level === 'debug' || level === 'info')) {
        return;
      }
      const timestamp = now();
      while (
        timestamps[0] !== undefined &&
        timestamp - timestamps[0] >= windowMs
      ) {
        timestamps.shift();
      }
      if (timestamps.length >= maxEvents) {
        return;
      }
      const message =
        typeof args[0] === 'string' ? args[0] : 'frontend log event';
      const code = /^[A-Z0-9_]+$/.test(message) ? message : 'UI_LOG';
      const second = args[1];
      const text =
        code === message && typeof second === 'string' ? second : message;
      const event = frontendLogEventSchema.parse({
        level,
        code,
        target,
        msg: text
      });
      timestamps.push(timestamp);
      void send([event]);
    };
  };
  // The level is a runtime preference owned by the application, never a browser
  // persistence mechanism. Passing `false` prevents loglevel from writing a
  // `loglevel:*` key to localStorage in the WebView/browser fallback.
  logger.setLevel(detailed ? 'debug' : 'warn', false);

  return {
    logger,
    setDetailed(value: boolean) {
      detailed = value;
      logger.setLevel(value ? 'debug' : 'warn', false);
    }
  };
}

export function isDetailedLoggingActive(
  value: unknown,
  now: number = Date.now()
): boolean {
  return typeof value === 'string' && Date.parse(value) > now;
}

const applicationLogger = createLogger('application');

export function setApplicationDetailedLogging(enabled: boolean): void {
  applicationLogger.setDetailed(enabled);
}

export function getApplicationLogger() {
  return applicationLogger.logger;
}

async function sendToBackend(
  events: readonly FrontendLogEvent[]
): Promise<void> {
  const result = await invokeValidated(
    'log_frontend',
    { events },
    z.void().or(z.null())
  );
  if (!result.ok) {
    return;
  }
}
