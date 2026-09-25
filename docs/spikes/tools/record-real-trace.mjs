// T019b: records one real IPC session from the actual sidecar (not the synthetic corpus of
// T045) as raw NDJSON, exactly as it leaves stdout. No new trace format: this is the same
// envelope contracts/ipc-protocol.md defines and packages/trace-fixtures/tools/replay.mjs
// already exercises against Rust, just captured live instead of replayed from a fixture.
//
// Must run from an elevated shell: MSR-backed sensors (level A) require the sidecar process
// itself to be elevated, and elevation is inherited from the process that spawns it.
//
// Usage:
//   node docs/spikes/tools/record-real-trace.mjs --sidecar <path-to-SensorAgent.exe> \
//     --scenario <name> --out <file.ndjson> [--duration-seconds 90] [--interval-ms 1000]
import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';

const args = parseArgs(process.argv.slice(2));
if (!args.sidecar || !args.scenario || !args.out) {
  process.stderr.write(
    'Usage: record-real-trace.mjs --sidecar <exe> --scenario <name> --out <file.ndjson> ' +
      '[--duration-seconds 90] [--interval-ms 1000]\n'
  );
  process.exitCode = 2;
  process.exit();
}

const durationSeconds = args['duration-seconds'] ? Number(args['duration-seconds']) : 90;
const intervalMs = args['interval-ms'] ? Number(args['interval-ms']) : 1000;
const sessionNonce = randomUUID();
let sequence = 0;

await mkdir(dirname(args.out), { recursive: true });
const out = createWriteStream(args.out, { flags: 'w' });

const child = spawn(args.sidecar, [], { stdio: ['pipe', 'pipe', 'pipe'] });
child.stderr.on('data', (chunk) => process.stderr.write(chunk));

const lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
const waiters = [];
lines.on('line', (line) => {
  out.write(`${line}\n`);
  const parsed = tryParse(line);
  if (parsed) {
    dispatch(parsed);
  }
});

function dispatch(message) {
  const index = waiters.findIndex((waiter) => waiter.type === message.type);
  if (index >= 0) {
    const [waiter] = waiters.splice(index, 1);
    waiter.resolve(message);
  }
}

function waitFor(type, timeoutMs = 10_000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      const index = waiters.findIndex((waiter) => waiter.resolve === resolve);
      if (index >= 0) waiters.splice(index, 1);
      reject(new Error(`Timed out waiting for "${type}".`));
    }, timeoutMs);
    waiters.push({
      type,
      resolve: (message) => {
        clearTimeout(timer);
        resolve(message);
      }
    });
  });
}

function send(type, payload) {
  sequence += 1;
  const envelope = {
    protocol_version: 1,
    session_nonce: sessionNonce,
    sequence,
    timestamp_utc: new Date().toISOString(),
    type,
    payload
  };
  child.stdin.write(`${JSON.stringify(envelope)}\n`);
}

// hello uses sequence 0: the sidecar adopts it as the session baseline (SidecarSession.HandleHello).
sequence = -1;
send('hello', { app_version: 'record-real-trace-spike', supported_protocols: [1] });
await waitFor('hello_ack');
await waitFor('capabilities');

send('start', { interval_ms: intervalMs, detail: 'representative' });
await waitFor('started');
process.stderr.write(
  `[${args.scenario}] recording for ${durationSeconds}s at ${intervalMs}ms -> ${args.out}\n`
);

await new Promise((resolve) => setTimeout(resolve, durationSeconds * 1000));

send('stop', {});
await waitFor('stopped').catch(() => {});
send('shutdown', {});

await new Promise((resolve) => {
  child.once('exit', resolve);
  setTimeout(resolve, 5_000);
});
out.end();
process.stderr.write(`[${args.scenario}] done.\n`);

function tryParse(line) {
  try {
    return JSON.parse(line);
  } catch {
    return null;
  }
}

function parseArgs(argv) {
  const result = {};
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i].startsWith('--')) {
      const key = argv[i].slice(2);
      const value = argv[i + 1];
      result[key] = value;
      i += 1;
    }
  }
  return result;
}
