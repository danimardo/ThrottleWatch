import { expect, test as base } from '@playwright/test';

export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript(() => {
      const listeners = new Map<string, Set<(payload: unknown) => void>>();
      const denseAnalysis = new URLSearchParams(window.location.search).has(
        'benchmark'
      );
      const shortcutFixture = new URLSearchParams(window.location.search).has(
        'shortcut-fixture'
      );
      const onboardingProfile = new URLSearchParams(window.location.search).get(
        'onboarding'
      );
      const historyFixture = new URLSearchParams(window.location.search).has(
        'with-history'
      );
      const activeFixture = new URLSearchParams(window.location.search).has(
        'active-session'
      );
      const corruptBackupFixture = new URLSearchParams(
        window.location.search
      ).has('corrupt-backup');
      let accessState = new URLSearchParams(window.location.search).get(
        'access'
      );
      const updatesProfile = new URLSearchParams(window.location.search).get(
        'updates'
      );
      const glassFixture = new URLSearchParams(window.location.search).get(
        'glass'
      );
      let imported = false;
      const updateState = {
        state: 'idle',
        enabled: updatesProfile !== null,
        current_version: '0.1.0',
        last_check: null as string | null,
        version: null as string | null,
        notes: null as string | null,
        downloaded: null as number | null,
        total: null as number | null,
        error_code: null as string | null,
        recoverable: null as boolean | null
      };
      const calls: Array<{ command: string; args?: Record<string, unknown> }> =
        [];
      const noArgumentCommands = new Set([
        'get_live_snapshot',
        'get_coverage',
        'recheck_coverage',
        'get_onboarding_state',
        'get_window_state',
        'get_preferences',
        'get_storage_usage',
        'export_corrupt_backup',
        'get_technical_summary',
        'get_third_party_notices',
        'open_logs_folder',
        'disable_advanced_access',
        'get_guided_preflight',
        'stop_guided',
        'cancel_export',
        'import_session',
        'get_cpu_topology',
        'get_update_state',
        'download_update',
        'install_update',
        'get_effective_glass_level'
      ]);
      const requestCommands = new Set([
        'set_onboarding_state',
        'delete_monitoring_data',
        'reset_application',
        'open_external_url',
        'set_preference',
        'set_window_state',
        'request_low_level_access',
        'start_guided',
        'get_analysis_window',
        'preview_export',
        'export',
        'set_tray_paused',
        'list_sessions',
        'get_session',
        'get_report',
        'delete_session',
        'set_session_reference',
        'reevaluate_report',
        'check_for_update',
        'confirm_close',
        'resolve_first_close',
        'report_glass_fps'
      ]);
      const validateCommandArguments = (
        command: string,
        args: Record<string, unknown> | undefined
      ): void => {
        if (noArgumentCommands.has(command)) {
          if (args !== undefined) {
            throw new Error(
              `E2E bridge: ${command} must not receive arguments`
            );
          }
          return;
        }
        if (command === 'log_frontend') {
          if (
            args === undefined ||
            Object.keys(args).length !== 1 ||
            !Array.isArray(args.events)
          ) {
            throw new Error(`E2E bridge: ${command} requires { events }`);
          }
          return;
        }
        if (requestCommands.has(command)) {
          if (
            args === undefined ||
            Object.keys(args).length !== 1 ||
            args.request === undefined
          ) {
            throw new Error(`E2E bridge: ${command} requires { request }`);
          }
          return;
        }
        throw new Error(`E2E bridge: unknown command ${command}`);
      };
      (
        window as unknown as { __THROTTLEWATCH_CALLS__: typeof calls }
      ).__THROTTLEWATCH_CALLS__ = calls;
      const emit = (event: string, payload: unknown) => {
        listeners.get(event)?.forEach((handler) => handler(payload));
      };
      // Lets a test simulate what Rust would emit on its own (`lifecycle:close-blocked`, an
      // update or session event outside its own command's flow): this harness runs against a
      // plain browser tab, never a real Tauri window, so nothing here can trigger those from an
      // actual native close.
      (
        window as unknown as { __THROTTLEWATCH_EMIT__: typeof emit }
      ).__THROTTLEWATCH_EMIT__ = emit;
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
        invoke: async (command: string, args?: Record<string, unknown>) => {
          validateCommandArguments(command, args);
          calls.push({ command, args });
          if (command === 'get_update_state') return { ...updateState };
          if (command === 'check_for_update') {
            if (!updateState.enabled) {
              throw { code: 'update.disabled', message_key: 'update.disabled' };
            }
            updateState.state = 'checking';
            emit('update:state-changed', { ...updateState });
            setTimeout(() => {
              updateState.state = 'available';
              updateState.version = '9.9.9';
              updateState.notes = 'Fixture release';
              updateState.last_check = '2026-09-21T08:00:00Z';
              emit('update:state-changed', { ...updateState });
            }, 30);
            return { ...updateState };
          }
          if (command === 'download_update') {
            updateState.state = 'downloading';
            updateState.downloaded = 40;
            updateState.total = 100;
            emit('update:state-changed', { ...updateState });
            setTimeout(() => {
              updateState.state = 'verified';
              updateState.downloaded = null;
              updateState.total = null;
              emit('update:state-changed', { ...updateState });
            }, 30);
            return { ...updateState };
          }
          if (command === 'install_update') {
            if (updatesProfile === 'blocked') {
              throw {
                code: 'update.blocked_by_running_operation',
                message_key: 'update.blocked.guided_test'
              };
            }
            updateState.state = 'installing';
            emit('update:state-changed', { ...updateState });
            return { ...updateState };
          }
          if (command === 'get_onboarding_state') {
            if (onboardingProfile === 'fresh') {
              return {
                flow_version: 1,
                last_slide: 1,
                status: 'pending',
                completed_at: null,
                last_seen_notice_version: 0
              };
            }
            if (onboardingProfile === 'midway') {
              return {
                flow_version: 1,
                last_slide: 3,
                status: 'pending',
                completed_at: null,
                last_seen_notice_version: 0
              };
            }
            return {
              flow_version: 1,
              last_slide: 5,
              status: 'completed',
              completed_at: '2026-09-20T10:00:00Z',
              last_seen_notice_version: 1
            };
          }
          if (accessState && command === 'disable_advanced_access') {
            accessState = 'denied';
          }
          if (accessState && command === 'request_low_level_access') {
            const action = (args?.request as { action?: string } | undefined)
              ?.action;
            accessState = 'available';
            return {
              state: 'install_requested',
              action,
              reboot_may_be_required: true
            };
          }
          if (
            accessState &&
            (command === 'get_coverage' ||
              command === 'recheck_coverage' ||
              command === 'disable_advanced_access')
          ) {
            return {
              tier: accessState === 'available' ? 'A' : 'B',
              confidence_ceiling:
                accessState === 'available' ? 'high' : 'medium',
              advanced_access: accessState,
              conclusion_key:
                accessState === 'available'
                  ? 'coverage.conclusion.a'
                  : 'coverage.conclusion.b',
              rows: []
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
          if (command === 'get_preferences' || command === 'set_preference') {
            const request = args?.request as
              { key?: string; value?: unknown } | undefined;
            const values: Record<string, unknown> = {
              'locale.mode': 'system',
              'appearance.theme': 'system',
              'appearance.motion': 'system',
              'appearance.glass': glassFixture ?? 'system',
              'sampling.profile': 'normal',
              'sampling.on_battery': 'keep',
              'sampling.per_core_history': false,
              'history.retention': '7d',
              'notifications.enabled': false,
              'notifications.quiet_period': null,
              'tray.monitoring_enabled': false,
              'lifecycle.close_action': 'unset',
              'startup.enabled': false,
              'startup.mode': 'window',
              'guided.duration': 'standard',
              'guided.require_ac': false,
              'guided.notify_on_finish': false,
              'privacy.anonymize_exports': true,
              'updates.enabled': false,
              'logging.detailed_until': null
            };
            if (request?.key) values[request.key] = request.value;
            return { schema_version: 1, values, adjusted: [] };
          }
          if (command === 'get_effective_glass_level') {
            const level = glassFixture ?? 'full';
            return {
              level:
                level === 'full' || level === 'reduced' || level === 'off'
                  ? level
                  : 'full'
            };
          }
          if (command === 'report_glass_fps') {
            return null;
          }
          if (command === 'get_storage_usage') {
            return {
              database_bytes: 4096,
              logs_bytes: 1024,
              total_bytes: 5120,
              session_count: historyFixture ? 2 : 0,
              corrupt_backup: corruptBackupFixture
                ? 'throttlewatch.db.corrupt-2026-09-22T00-00-00Z'
                : null
            };
          }
          if (command === 'export_corrupt_backup') {
            return null;
          }
          if (command === 'get_technical_summary') {
            return {
              text: 'ThrottleWatch e2e fixture\nCollector: running\nCoverage tier: A'
            };
          }
          if (command === 'get_third_party_notices') {
            return { entries: [] };
          }
          if (command === 'preview_export') {
            const request = args?.request as
              { format?: 'csv' | 'json' } | undefined;
            return {
              format: request?.format ?? 'json',
              included_fields: ['session', 'samples', 'events'],
              excluded_fields: ['cpu identifiers', 'paths'],
              estimated_bytes: 2048,
              proposed_file_name: 'throttlewatch-session.json'
            };
          }
          if (command === 'export') {
            return { bytes: 2048, anonymized: true, warnings: [] };
          }
          if (command === 'import_session') {
            imported = true;
            return {
              session_id: 'imported-session',
              schema_version: 1,
              migrated: false,
              warnings: []
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
          if (
            command === 'list_sessions' &&
            (shortcutFixture || historyFixture || activeFixture)
          ) {
            return {
              sessions: [
                {
                  session_id: activeFixture
                    ? 'active-session'
                    : historyFixture
                      ? 'history-session'
                      : 'shortcut-session',
                  kind: 'guided',
                  status: activeFixture ? 'active' : 'completed',
                  started_at: '2026-09-20T10:00:00Z',
                  ended_at: activeFixture ? null : '2026-09-20T10:05:00Z',
                  duration_ms: activeFixture ? null : 300_000,
                  coverage_tier: 'A',
                  is_reference: false,
                  frame_count: 300,
                  report_classification: activeFixture ? null : 'normal'
                }
              ],
              next_cursor: null
            };
          }
          if (command === 'list_sessions') {
            return {
              sessions: imported
                ? [
                    {
                      session_id: 'imported-session',
                      kind: 'imported',
                      status: 'imported',
                      started_at: '2026-09-18T09:00:00Z',
                      ended_at: '2026-09-18T09:06:00Z',
                      duration_ms: 360_000,
                      coverage_tier: 'B',
                      is_reference: false,
                      frame_count: 200,
                      report_classification: 'thermal_confirmed'
                    }
                  ]
                : [],
              next_cursor: null
            };
          }
          if (command === 'get_session' || command === 'get_report') {
            const request = args?.request as
              { session_id?: string } | undefined;
            if (request?.session_id === 'imported-session') {
              const report = {
                classification: 'thermal_confirmed',
                severity: 'below_base',
                events: [],
                cooling_potential: null
              };
              return command === 'get_session'
                ? {
                    summary: {
                      session_id: 'imported-session',
                      kind: 'imported',
                      status: 'imported',
                      started_at: '2026-09-18T09:00:00Z',
                      ended_at: '2026-09-18T09:06:00Z',
                      duration_ms: 360_000,
                      coverage_tier: 'B',
                      is_reference: false,
                      frame_count: 200,
                      report_classification: 'thermal_confirmed'
                    },
                    report
                  }
                : {
                    session_id: 'imported-session',
                    schema_version: 1,
                    report,
                    frozen_at: '2026-09-18T09:06:00Z'
                  };
            }
            return command === 'get_session'
              ? {
                  summary: {
                    session_id: 'history-session',
                    kind: 'guided',
                    status: 'completed',
                    started_at: '2026-09-20T10:00:00Z',
                    ended_at: '2026-09-20T10:05:00Z',
                    duration_ms: 300_000,
                    coverage_tier: 'A',
                    is_reference: false,
                    frame_count: 300,
                    report_classification: 'normal'
                  },
                  report: {
                    classification: 'normal',
                    events: [],
                    cooling_potential: null
                  }
                }
              : {
                  session_id: 'history-session',
                  schema_version: 1,
                  report: { classification: 'normal', events: [] },
                  frozen_at: '2026-09-20T10:05:00Z'
                };
          }
          if (command === 'reevaluate_report') {
            return {
              session_id: 'imported-session',
              schema_version: 1,
              // Re-run with today's rules, this same trace no longer confirms a thermal limit:
              // exercises the "different from the original" branch of the reevaluation panel.
              report: { classification: 'normal', events: [] },
              frozen_at: null
            };
          }
          if (
            command === 'delete_session' ||
            command === 'set_session_reference' ||
            command === 'confirm_close' ||
            command === 'resolve_first_close'
          ) {
            return null;
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
    const storageKeys = await page.evaluate(() =>
      Array.from({ length: localStorage.length }, (_, index) =>
        localStorage.key(index)
      ).filter((key): key is string => key !== null)
    );
    if (new URL(page.url()).searchParams.has('onboarding')) {
      expect(storageKeys).toEqual(['throttlewatch.onboarding-state']);
    } else {
      expect(storageKeys).toEqual([]);
    }
    expect(allowedOrigins.size).toBe(1);
    if (errors.length > 0) throw new Error(`E2E errors: ${errors.join('; ')}`);
  }
});

export { expect } from '@playwright/test';
