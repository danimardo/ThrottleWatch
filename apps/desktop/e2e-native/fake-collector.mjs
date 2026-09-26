#!/usr/bin/env node
// Test double for the collector (`SensorAgent.exe`), for the native E2E suite (T181).
//
// The real collector only starts from a manifest signed by a trusted key, and the private half of
// the development key lives outside the repository on purpose, so a CI runner can never launch it.
// Every scenario that needs live telemetry — the full-disk one records samples, so it needs
// samples — was therefore unreachable there. The `e2e` build starts this instead when it is given
// `TW_DEV_COLLECTOR_CMD` (`src-tauri/src/dev_faults.rs`); nothing here weakens the check the real
// collector goes through, it simply is not part of that path.
//
// It speaks `contracts/ipc-protocol.md`: NDJSON on stdin/stdout, `hello` -> `hello_ack` +
// `capabilities`, `start` -> `started` and then a `sample` per interval. The `session_nonce` comes
// in the `hello` and is echoed back, exactly like the real one. It reports no low-level access, so
// coverage is level B and no thermal-limit flag is ever emitted — enough to exercise recording and
// the live screens, and nothing pretends to be more than that: the CPU is named as a test double.
import { createInterface } from 'node:readline';

const PROTOCOL_VERSION = 1;
const MIN_INTERVAL_MS = 250;
const MAX_INTERVAL_MS = 10_000;

const CAPABILITIES = {
  cpu: {
    vendor: 'intel',
    display_name: 'Test double CPU (fake collector)',
    logical_processors: 8,
    physical_cores: 4,
    hybrid: false,
    virtualized: false
  },
  groups: [{ id: 'all', kind: 'homogeneous', logical_count: 8 }],
  sensors: [
    {
      id: 'cpu.package.load',
      source_id: 'fake-load',
      source_name: 'CPU Total',
      metric: 'load',
      scope: 'package',
      unit: 'percent',
      quality: 'direct'
    },
    {
      id: 'cpu.package.temp',
      source_id: 'fake-temp',
      source_name: 'CPU Package',
      metric: 'temperature',
      scope: 'package',
      unit: 'celsius',
      quality: 'direct'
    },
    {
      id: 'cpu.package.power',
      source_id: 'fake-power',
      source_name: 'CPU Package',
      metric: 'power',
      scope: 'package',
      unit: 'watt',
      quality: 'direct'
    }
  ]
};

let nonce = null;
let sequence = 0;
let timer = null;
let tick = 0;

function send(type, payload) {
  process.stdout.write(
    `${JSON.stringify({
      protocol_version: PROTOCOL_VERSION,
      session_nonce: nonce,
      sequence: sequence++,
      timestamp_utc: new Date().toISOString(),
      type,
      payload
    })}\n`
  );
}

const round = (value) => Math.round(value * 10) / 10;

/** A gentle, deterministic wave: the values move, never reach a limit, never look like a fault. */
function sample() {
  tick += 1;
  const wave = Math.sin(tick / 6);
  return {
    monotonic_ms: Math.round(process.uptime() * 1000),
    duration_ms: 1,
    values: [
      {
        sensor_id: 'cpu.package.load',
        number: round(35 + 20 * wave),
        status: 'ok'
      },
      {
        sensor_id: 'cpu.package.temp',
        number: round(58 + 6 * wave),
        status: 'ok'
      },
      {
        sensor_id: 'cpu.package.power',
        number: round(22 + 8 * wave),
        status: 'ok'
      }
    ]
  };
}

function clampInterval(requested) {
  const value = Number.isFinite(requested) ? requested : 1000;
  return Math.min(
    MAX_INTERVAL_MS,
    Math.max(MIN_INTERVAL_MS, Math.round(value))
  );
}

function stopSampling() {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }
}

function startSampling(intervalMs, detail) {
  stopSampling();
  send('started', { interval_ms: intervalMs, detail });
  timer = setInterval(() => send('sample', sample()), intervalMs);
}

let detail = 'representative';

createInterface({ input: process.stdin }).on('line', (line) => {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return; // the host never sends anything else; ignoring is what the real collector does too
  }

  switch (message.type) {
    case 'hello':
      nonce = message.session_nonce;
      send('hello_ack', {
        agent_version: 'fake-collector',
        selected_protocol: PROTOCOL_VERSION,
        runtime: 'node',
        low_level_access: {
          state: 'missing',
          provider: null,
          details_code: null
        }
      });
      send('capabilities', CAPABILITIES);
      break;
    case 'start':
      detail = message.payload?.detail ?? detail;
      startSampling(clampInterval(message.payload?.interval_ms), detail);
      break;
    case 'set_rate':
      startSampling(clampInterval(message.payload?.interval_ms), detail);
      break;
    case 'snapshot':
      send('sample', sample());
      break;
    case 'stop':
      stopSampling();
      send('stopped', {});
      break;
    case 'shutdown':
      stopSampling();
      process.exit(0);
      break;
    default:
      break;
  }
});

// EOF on stdin is the host's orderly shutdown, same as for the real collector.
process.stdin.on('end', () => {
  stopSampling();
  process.exit(0);
});
