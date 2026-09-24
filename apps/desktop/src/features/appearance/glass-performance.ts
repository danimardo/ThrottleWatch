import { invokeValidated } from '../../lib/bridge';
import { commandResponseSchemas } from '../../lib/bridge/schemas';

/**
 * T131's automatic glass degradation needs a real fps reading, but a continuous
 * `requestAnimationFrame` counter has a measurable cost of its own (docs/spikes/glass-cost.md,
 * finding 5: ~4 % of one core just from the counter). This measures fps for one short window,
 * reports it to Rust, then waits before measuring again — never running continuously.
 */
const SAMPLE_WINDOW_MS = 3000;
const SAMPLE_INTERVAL_MS = 15000;

function measureFpsOnce(onSample: (fps: number) => void): void {
  if (typeof requestAnimationFrame === 'undefined') return;
  let frames = 0;
  const start = performance.now();
  const loop = (): void => {
    frames += 1;
    const elapsed = performance.now() - start;
    if (elapsed >= SAMPLE_WINDOW_MS) {
      onSample((frames * 1000) / elapsed);
      return;
    }
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

/** Starts the periodic short-window fps sampling loop; returns a function that stops it. */
export function startGlassPerformanceReporting(): () => void {
  let stopped = false;
  let timeoutId: ReturnType<typeof window.setTimeout> | undefined;

  const scheduleNext = (): void => {
    if (stopped || typeof window === 'undefined') return;
    timeoutId = window.setTimeout(() => {
      if (stopped) return;
      measureFpsOnce((fps) => {
        void invokeValidated(
          'report_glass_fps',
          { request: { fps } },
          commandResponseSchemas.report_glass_fps
        );
        scheduleNext();
      });
    }, SAMPLE_INTERVAL_MS);
  };

  scheduleNext();
  return () => {
    stopped = true;
    if (timeoutId !== undefined) window.clearTimeout(timeoutId);
  };
}
