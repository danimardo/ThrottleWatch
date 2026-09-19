import { z } from 'zod';

const envSchema = z.object({
  PUBLIC_LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info')
});

export type PublicEnv = z.infer<typeof envSchema>;

export function parsePublicEnv(source: Record<string, unknown>): PublicEnv {
  return envSchema.parse(source);
}

export function readPublicEnv(): PublicEnv {
  return parsePublicEnv({ PUBLIC_LOG_LEVEL: import.meta.env.PUBLIC_LOG_LEVEL });
}
