import { z } from 'zod';

// Constitution XVII: development logs at `debug` unless told otherwise; `trace` is accepted here
// because this variable only acts in a development build (vite.config.ts rejects it in production).
const envSchema = z.object({
  PUBLIC_LOG_LEVEL: z
    .enum(['trace', 'debug', 'info', 'warn', 'error'])
    .default('debug')
});

export type PublicEnv = z.infer<typeof envSchema>;

export function parsePublicEnv(source: Record<string, unknown>): PublicEnv {
  return envSchema.parse(source);
}

export function readPublicEnv(): PublicEnv {
  return parsePublicEnv({ PUBLIC_LOG_LEVEL: import.meta.env.PUBLIC_LOG_LEVEL });
}
