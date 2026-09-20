import { expect, test as base } from '@playwright/test';

export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript(() => {
      const listeners = new Map<string, Set<(payload: unknown) => void>>();
      const denseAnalysis = new URLSearchParams(window.location.search).has(
        'benchmark'
      );
      const emit = (event: string, payload: unknown) => {
        listeners.get(event)?.forEach((handler) => handler(payload));
      };
      const guided = (phase: string) => ({
        phase,
        elapsed_ms: 0,
        remaining_ms: phase === 'rest' ? 60_000 : 180_000,
        reason_key: null,
        temperature_c: 72,
        thermal_limit_c: 95,
        active_clock_mhz: 3800,
        base_clock_mhz: 3500,
        throughput_ops_s: 4_000_000,
        progress_percent: 0
      });
      const analysisWindow = () => {
        const points = denseAnalysis
          ? Array.from({ length: 3000 }, (_, index) => ({
              start_ms: index * 100,
              end_ms: (index + 1) * 100,
              first: index,
              last: index + 1,
              min: index,
              max: index + 1,
              average: index + 0.5,
              quality: index % 997 === 0 ? 'reduced' : 'complete',
              gap: false
            }))
          : [
              {
                start_ms: 0,
                end_ms: 500,
                first: 1,
                last: 2,
                min: 1,
                max: 2,
                average: 1.5,
                quality: 'complete',
                gap: false
              },
              {
                start_ms: 500,
                end_ms: 750,
                first: 2,
                last: 2,
                min: 2,
                max: 2,
                average: 2,
                quality: 'reduced',
                gap: false
              },
              {
                start_ms: 750,
                end_ms: 900,
                first: 3,
                last: 3,
                min: 3,
                max: 3,
                average: 3,
                quality: 'reduced',
                gap: false
              },
              {
                start_ms: 900,
                end_ms: 1000,
                first: null,
                last: null,
                min: null,
                max: null,
                average: null,
                quality: 'missing',
                gap: true
              }
            ];
        return {
          session_id: 'fixture-session',
          start_ms: 0,
          end_ms: denseAnalysis ? 300_000 : 1000,
          is_aggregated: true,
          tracks: ['temperature', 'clock', 'load', 'power'].map((kind) => ({
            kind,
            points
          })),
          events: [
            {
              id: 'event-1',
              kind: 'thermal',
              start_ms: denseAnalysis ? 100_000 : 250,
              end_ms: denseAnalysis ? 180_000 : 750,
              label: 'Thermal limit',
              informational: false
            },
            ...(denseAnalysis
              ? [
                  {
                    id: 'event-2',
                    kind: 'electrical',
                    start_ms: 140_000,
                    end_ms: 220_000,
                    label: 'Power cap',
                    informational: false
                  }
                ]
              : [])
          ]
        };
      };
      window.__THROTTLEWATCH_FAKE_BRIDGE__ = {
        invoke: async (command: string) => {
          if (command === 'get_onboarding_state') {
            return {
              flow_version: 1,
              last_slide: 5,
              status: 'completed',
              completed_at: '2026-09-20T10:00:00Z',
              last_seen_notice_version: 1
            };
          }
          if (command === 'get_coverage' || command === 'recheck_coverage') {
            return {
              tier: 'A',
              confidence_ceiling: 'high',
              advanced_access: 'not_needed',
              conclusion_key: 'coverage.conclusion.a',
              rows: []
            };
          }
          if (command === 'get_guided_preflight') {
            return {
              sensors: true,
              ac_power: true,
              profile: true,
              disk_space: true,
              generator: true,
              require_ac: true
            };
          }
          if (command === 'get_cpu_topology') {
            return {
              cores: [0, 1, 2, 3].map((index) => ({
                id: `core-${index}`,
                index,
                group: index < 2 ? 'p' : 'e',
                temperature_c: 60 + index,
                clock_mhz: 3900 - index * 100,
                load_percent: 40 + index,
                throttling: false
              }))
            };
          }
          if (command === 'start_guided') {
            const phase = guided('rest');
            emit('guided:phase', phase);
            return phase;
          }
          if (command === 'stop_guided') {
            const phase = {
              ...guided('cancelled'),
              reason_key: 'guided.user_requested'
            };
            emit('guided:phase', phase);
            return phase;
          }
          if (command === 'get_analysis_window') {
            return analysisWindow();
          }
          return {};
        },
        listen: async (event: string, handler: (payload: unknown) => void) => {
          const handlers =
            listeners.get(event) ?? new Set<(payload: unknown) => void>();
          handlers.add(handler);
          listeners.set(event, handlers);
          return () => handlers.delete(handler);
        }
      };
    });
    const errors: string[] = [];
    const allowedOrigins = new Set(['http://127.0.0.1:4173']);
    await page.route('**/*', async (route) => {
      const origin = new URL(route.request().url()).origin;
      if (
        origin !== 'http://127.0.0.1:4173' &&
        origin !== 'http://localhost:4173'
      ) {
        throw new Error(`Unexpected external origin: ${origin}`);
      }
      await route.continue();
    });
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    await use(page);
    expect(await page.evaluate(() => localStorage.length)).toBe(0);
    expect(allowedOrigins.size).toBe(1);
    if (errors.length > 0) throw new Error(`E2E errors: ${errors.join('; ')}`);
  }
});

export { expect } from '@playwright/test';
