import { expect, test } from './fixtures';

test('@critical guided diagnosis shows phases and stops with Ctrl+Shift+X', async ({
  page
}) => {
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await expect(
    page.getByRole('heading', { name: /guided diagnostic|diagnóstico guiado/i })
  ).toBeVisible();
  await page.getByRole('button', { name: /start|iniciar/i }).click();
  await expect(
    page.getByRole('button', { name: /skip rest|omitir reposo/i })
  ).toBeVisible();
  await page.keyboard.press('Control+Shift+X');
  await expect(
    page.getByText(/incomplete test|prueba incompleta/i)
  ).toBeVisible();
});

test('@critical guided session remains stoppable after navigation and protects close', async ({
  page
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /start|iniciar/i }).click();
  await page.getByRole('button', { name: /skip rest|omitir reposo/i }).click();

  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  const globalStop = page.getByRole('button', {
    name: /stop now|detener ahora/i
  });
  await expect(globalStop).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(globalStop).toBeVisible();

  await page.locator('button.close').click();
  await expect(
    page.getByRole('button', { name: /stop and exit|detener y salir/i })
  ).toBeVisible();
  await page.getByRole('button', { name: /cancel|cancelar/i }).click();
  await expect(
    page.getByRole('button', { name: /stop and exit|detener y salir/i })
  ).toBeHidden();

  await globalStop.click();
  await expect(globalStop).toBeHidden();
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await expect(
    page.getByText(/incomplete test|prueba incompleta/i)
  ).toBeVisible();
});

test('@critical guided diagnosis also works at compact size', async ({
  page
}) => {
  await page.setViewportSize({ width: 480, height: 600 });
  await page.goto('/');
  await page.getByRole('button', { name: /more|más/i }).click();
  await page
    .getByRole('menuitem', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /start|iniciar/i }).click();
  await expect(
    page.getByRole('button', { name: /skip rest|omitir reposo/i })
  ).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('button', { name: /skip rest|omitir reposo/i })
  ).toBeVisible();
  await page.keyboard.press('Control+Shift+X');
  await expect(
    page.getByText(/incomplete test|prueba incompleta/i)
  ).toBeVisible();
});

test('@critical analysis renders four tracks, gaps and event evidence', async ({
  page
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  await expect(
    page
      .getByRole('group', { name: /analysis tracks|pistas del análisis/i })
      .first()
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Thermal limit' })
  ).toBeVisible();
  await expect(
    page.getByRole('slider', { name: /time cursor|cursor temporal/i })
  ).toHaveAttribute('aria-valuenow', '0');
  await expect(page.locator('path[stroke-dasharray="3 3"]')).toHaveCount(4);
  await expect(page.getByText(/^Evidence$|^Evidencia$/i)).toBeVisible();
  await page
    .getByRole('slider', { name: /time cursor|cursor temporal/i })
    .press('Home');
  await page
    .getByRole('slider', { name: /time cursor|cursor temporal/i })
    .press('Enter');
  await page
    .getByRole('slider', { name: /time cursor|cursor temporal/i })
    .press('End');
  await page
    .getByRole('slider', { name: /time cursor|cursor temporal/i })
    .press('Enter');
  await expect(
    page.getByText(/Selected range|Rango seleccionado/i, { exact: true })
  ).toBeVisible();
  await expect(page.locator('table')).toBeAttached();
  await expect(
    page.getByRole('slider', { name: /time cursor|cursor temporal/i })
  ).toHaveAttribute('aria-valuenow', '3');
  await page.screenshot({
    path: 'test-results/analysis-guided.png',
    fullPage: true
  });
});

test('@visual analysis benchmark renders four 3000-point tracks', async ({
  page
}) => {
  await page.goto('/?benchmark=analysis');
  const started = Date.now();
  await page.getByRole('button', { name: /analysis|análisis/i }).click();
  await expect(page.getByRole('button', { name: 'temperature' })).toBeVisible();
  const elapsedMs = Date.now() - started;
  await expect(
    page.getByRole('button', { name: 'Thermal limit' })
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Power cap' })).toBeVisible();
  await expect(page.locator('svg path')).not.toHaveCount(0);
  await test.info().attach('analysis-chart-benchmark', {
    body: JSON.stringify({
      tracks: 4,
      points_per_track: 3000,
      elapsed_ms: elapsedMs
    }),
    contentType: 'application/json'
  });
  expect(elapsedMs).toBeLessThan(2_000);
});

test('@critical CPU topology keeps core selection while changing metric', async ({
  page
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /cpu/i }).click();
  await expect(page.getByRole('heading', { name: 'CPU' })).toBeVisible();
  await expect(page.locator('.group-label', { hasText: 'P' })).toBeVisible();
  await expect(page.locator('.group-label', { hasText: 'E' })).toBeVisible();

  const firstCore = page
    .locator('button[aria-label*="Core 0"], button[aria-label*="Núcleo 0"]')
    .first();
  await firstCore.click();
  await expect(firstCore).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: /frequency|frecuencia/i }).click();
  await expect(firstCore).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByText('3900 MHz', { exact: true })).toBeVisible();
});

test('@critical guided result shows the verdict, sets the reference and opens the report', async ({
  page
}) => {
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /start|iniciar/i }).click();

  // What Rust emits when the run ends: the session it saved, then the final phase.
  await page.evaluate(() => {
    const emit = (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__;
    emit('guided:finished', { session_id: 'history-session' });
    emit('guided:phase', {
      phase: 'result',
      elapsed_ms: 300_000,
      remaining_ms: 0,
      reason_key: null,
      temperature_c: 70,
      thermal_limit_c: 95,
      active_clock_mhz: 3900,
      base_clock_mhz: 3600,
      throughput_ops_s: 1000,
      progress_percent: 100
    });
  });

  await expect(
    page.getByText(/no limitation detected|sin limitación detectada/i)
  ).toBeVisible();
  await page
    .getByRole('button', { name: /use as reference|usar como referencia/i })
    .click();
  const calls = await page.evaluate(
    () =>
      (
        window as unknown as {
          __THROTTLEWATCH_CALLS__: Array<{ command: string }>;
        }
      ).__THROTTLEWATCH_CALLS__
  );
  expect(calls.map(({ command }) => command)).toContain(
    'set_session_reference'
  );

  await page.getByRole('button', { name: /view report|ver informe/i }).click();
  await expect(
    page.getByRole('main', { name: /sessions|sesiones/i })
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /back to sessions|volver a sesiones/i })
  ).toBeVisible();

  // Coming back to the guided screen starts over instead of showing the finished run again.
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await expect(
    page.getByRole('button', { name: /^(start|iniciar)$/i })
  ).toBeVisible();
});

test('@critical guided start uses the duration and AC choices from Settings', async ({
  page
}) => {
  await page.goto('/');
  await page.keyboard.press('Control+6');
  await page.evaluate(async () => {
    const invoke = (
      window as unknown as {
        __THROTTLEWATCH_FAKE_BRIDGE__: {
          invoke: (command: string, args: unknown) => Promise<unknown>;
        };
      }
    ).__THROTTLEWATCH_FAKE_BRIDGE__;
    await invoke.invoke('set_preference', {
      request: { key: 'guided.duration', value: 'long' }
    });
    await invoke.invoke('set_preference', {
      request: { key: 'guided.require_ac', value: true }
    });
  });

  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /^(start|iniciar)$/i }).click();

  const start = await page.evaluate(
    () =>
      (
        window as unknown as {
          __THROTTLEWATCH_CALLS__: Array<{
            command: string;
            args?: { request?: Record<string, unknown> };
          }>;
        }
      ).__THROTTLEWATCH_CALLS__.find((call) => call.command === 'start_guided')
        ?.args?.request
  );
  expect(start).toMatchObject({ profile: 'long', require_ac: true });
});

test('@critical guided recovers after a sensor-lost stop and Cerrar, without leftover state', async ({
  page
}) => {
  await page.goto('/');
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /start|iniciar/i }).click();

  await page.evaluate(() => {
    const emit = (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__;
    emit('guided:phase', {
      phase: 'sensor_lost',
      elapsed_ms: 60_000,
      remaining_ms: null,
      reason_key: 'guided.sensor_lost',
      temperature_c: null,
      thermal_limit_c: null,
      active_clock_mhz: null,
      base_clock_mhz: null,
      throughput_ops_s: null,
      progress_percent: null
    });
  });
  await expect(page.getByText(/sensor lost|sensor perdido/i)).toBeVisible();

  await page
    .getByRole('main', { name: /guided diagnostic|diagnóstico guiado/i })
    .getByRole('button', { name: /^(close|cerrar)$/i })
    .click();
  // Cerrar takes a non-result finish back to "Ahora".
  await expect(page.getByRole('main', { name: /now|ahora/i })).toBeVisible();

  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  const startAgain = page.getByRole('button', { name: /^(start|iniciar)$/i });
  await expect(startAgain).toBeVisible();
  await startAgain.click();
  await expect(
    page.getByRole('button', { name: /skip rest|omitir reposo/i })
  ).toBeVisible();
});

test('@critical a start_guided rejection the client-side checks did not predict still says so', async ({
  page
}) => {
  await page.goto('/');
  await page.evaluate(() => {
    const bridge = (
      window as unknown as {
        __THROTTLEWATCH_FAKE_BRIDGE__: {
          invoke: (command: string, args: unknown) => Promise<unknown>;
        };
      }
    ).__THROTTLEWATCH_FAKE_BRIDGE__;
    const originalInvoke = bridge.invoke.bind(bridge);
    let attempt = 0;
    bridge.invoke = async (command: string, args: unknown) => {
      if (command === 'start_guided') {
        attempt += 1;
        if (attempt > 1) {
          throw {
            code: 'LOW_LEVEL_ACCESS_OPERATION_FAILED',
            message_key: 'access.operation_failed'
          };
        }
      }
      return originalInvoke(command, args);
    };
  });

  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  await page.getByRole('button', { name: /start|iniciar/i }).click();
  await page.evaluate(() => {
    const emit = (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__;
    emit('guided:phase', {
      phase: 'sensor_lost',
      elapsed_ms: 60_000,
      remaining_ms: null,
      reason_key: 'guided.sensor_lost',
      temperature_c: null,
      thermal_limit_c: null,
      active_clock_mhz: null,
      base_clock_mhz: null,
      throughput_ops_s: null,
      progress_percent: null
    });
  });
  await page
    .getByRole('main', { name: /guided diagnostic|diagnóstico guiado/i })
    .getByRole('button', { name: /^(close|cerrar)$/i })
    .click();
  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();

  const startButton = page.getByRole('button', { name: /^(start|iniciar)$/i });
  await expect(startButton).toBeVisible();
  await startButton.click();
  // The client-side checks all passed (the fake bridge's `get_guided_preflight` never changed),
  // so this is the one case the new proactive checklist cannot catch: the server rejected the
  // attempt for a reason only it could see. The person must still be told, not left looking at
  // the same "Start" screen with no visible change.
  await expect(
    page.getByText(/could not be completed|no se pudo completar/i)
  ).toBeVisible();
});

test('@critical a still-failing sensor after Cerrar shows the checklist up front, not a blank retry', async ({
  page
}) => {
  await page.goto('/');
  await page.evaluate(() => {
    const bridge = (
      window as unknown as {
        __THROTTLEWATCH_FAKE_BRIDGE__: {
          invoke: (command: string, args: unknown) => Promise<unknown>;
        };
      }
    ).__THROTTLEWATCH_FAKE_BRIDGE__;
    // Captured once, on `window`, so the later override can restore the real thing instead of
    // re-wrapping whatever override happens to be installed at that point.
    const holder = window as unknown as {
      __ORIGINAL_INVOKE__?: typeof bridge.invoke;
    };
    holder.__ORIGINAL_INVOKE__ ??= bridge.invoke.bind(bridge);
    const original = holder.__ORIGINAL_INVOKE__;
    bridge.invoke = async (command: string, args: unknown) => {
      if (command === 'get_guided_preflight') {
        return {
          sensors: false,
          ac_power: true,
          profile: true,
          disk_space: true,
          generator: true,
          require_ac: true
        };
      }
      return original(command, args);
    };
  });

  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();

  // The checklist shows the failing check up front — no need to click Start first to find out.
  await expect(page.getByText(/^(sensors|sensores)$/i)).toBeVisible();
  await expect(
    page.getByText(/review the conditions|revisa las condiciones/i)
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /^(start|iniciar)$/i })
  ).toBeHidden();

  const recheck = page.getByRole('button', {
    name: /check again|comprobar de nuevo/i
  });
  await expect(recheck).toBeVisible();
  await recheck.click();
  // Recovers once the sensor genuinely reports again: swap back to the real (pristine) invoke
  // captured above, so the fake bridge answers truthfully on the next check.
  await page.evaluate(() => {
    const holder = window as unknown as {
      __THROTTLEWATCH_FAKE_BRIDGE__: {
        invoke: (command: string, args: unknown) => Promise<unknown>;
      };
      __ORIGINAL_INVOKE__: (command: string, args: unknown) => Promise<unknown>;
    };
    holder.__THROTTLEWATCH_FAKE_BRIDGE__.invoke = holder.__ORIGINAL_INVOKE__;
  });
  await recheck.click();
  await expect(
    page.getByRole('button', { name: /^(start|iniciar)$/i })
  ).toBeVisible();
});

test('@critical Repetir on a failed restart shows the checklist instead of doing nothing', async ({
  page
}) => {
  // T186/T187 (2026-09-28): the backend correctly rejects a restart when the sensors it needs
  // are still gone (a collector that just lost elevation), but the screen used to stay frozen on
  // the terminal phase with no visible reaction — "Repetir" read as doing nothing.
  await page.goto('/');
  await page.evaluate(() => {
    const bridge = (
      window as unknown as {
        __THROTTLEWATCH_FAKE_BRIDGE__: {
          invoke: (command: string, args: unknown) => Promise<unknown>;
        };
      }
    ).__THROTTLEWATCH_FAKE_BRIDGE__;
    const holder = window as unknown as {
      __ORIGINAL_INVOKE__?: typeof bridge.invoke;
    };
    holder.__ORIGINAL_INVOKE__ ??= bridge.invoke.bind(bridge);
    const original = holder.__ORIGINAL_INVOKE__;
    bridge.invoke = async (command: string, args: unknown) => {
      if (command === 'get_guided_preflight') {
        return {
          sensors: false,
          ac_power: true,
          profile: true,
          disk_space: true,
          generator: true,
          require_ac: true
        };
      }
      if (command === 'start_guided') {
        throw {
          code: 'LOW_LEVEL_ACCESS_OPERATION_FAILED',
          message_key: 'access.operation_failed'
        };
      }
      return original(command, args);
    };
  });

  await page
    .getByRole('button', { name: /guided diagnostic|diagnóstico guiado/i })
    .click();
  // Reaches the terminal "sensor lost" screen directly, as the real backend would once a
  // running test loses its sensors mid-run — no need to wait out the phase timers.
  await page.evaluate(() => {
    const emit = (
      window as unknown as {
        __THROTTLEWATCH_EMIT__: (event: string, payload: unknown) => void;
      }
    ).__THROTTLEWATCH_EMIT__;
    emit('guided:phase', {
      phase: 'sensor_lost',
      elapsed_ms: 14_000,
      remaining_ms: null,
      reason_key: 'guided.sensor_lost',
      temperature_c: null,
      thermal_limit_c: null,
      active_clock_mhz: null,
      base_clock_mhz: null,
      throughput_ops_s: null,
      progress_percent: null
    });
  });
  await expect(page.getByText(/sensor lost|sensor perdido/i)).toBeVisible();

  await page.getByRole('button', { name: /try again|repetir/i }).click();

  // The rejected restart shows why, instead of leaving the stale "sensor lost" screen untouched.
  await expect(page.getByText(/^(sensors|sensores)$/i)).toBeVisible();
  await expect(
    page.getByText(/review the conditions|revisa las condiciones/i)
  ).toBeVisible();
});
