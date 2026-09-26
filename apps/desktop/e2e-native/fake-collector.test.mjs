// The fake collector is only worth having if it speaks the protocol the host validates, so this
// checks it on its own, without the application: `node --test e2e-native/fake-collector.test.mjs`.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('./fake-collector.mjs', import.meta.url));
const NONCE = 'test-nonce-123';

/** Starts the collector and gives back a way to send lines and wait for the next message. */
function launch() {
  const child = spawn(process.execPath, [script], {
    stdio: ['pipe', 'pipe', 'inherit']
  });
  const received = [];
  const waiting = [];
  createInterface({ input: child.stdout }).on('line', (line) => {
    const message = JSON.parse(line);
    const waiter = waiting.shift();
    if (waiter) waiter(message);
    else received.push(message);
  });
  return {
    child,
    send(type, payload = {}) {
      child.stdin.write(
        `${JSON.stringify({
          protocol_version: 1,
          session_nonce: NONCE,
          sequence: 0,
          timestamp_utc: new Date().toISOString(),
          type,
          payload
        })}\n`
      );
    },
    next(timeoutMs = 5000) {
      if (received.length > 0) return Promise.resolve(received.shift());
      return new Promise((resolve, reject) => {
        const timer = setTimeout(
          () => reject(new Error('no message from the fake collector')),
          timeoutMs
        );
        waiting.push((message) => {
          clearTimeout(timer);
          resolve(message);
        });
      });
    }
  };
}

test('it answers hello with hello_ack and capabilities, echoing the nonce', async () => {
  const collector = launch();
  try {
    collector.send('hello', { app_version: '0.1.0', supported_protocols: [1] });

    const ack = await collector.next();
    assert.equal(ack.type, 'hello_ack');
    assert.equal(ack.session_nonce, NONCE);
    assert.equal(ack.protocol_version, 1);
    assert.equal(ack.payload.selected_protocol, 1);
    // No low-level access: coverage stays at level B and no limit flag is ever claimed.
    assert.equal(ack.payload.low_level_access.state, 'missing');

    const capabilities = await collector.next();
    assert.equal(capabilities.type, 'capabilities');
    assert.equal(capabilities.session_nonce, NONCE);
    assert.ok(capabilities.payload.cpu.logical_processors > 0);
    assert.match(capabilities.payload.cpu.display_name, /test double/i);
    assert.deepEqual(
      capabilities.payload.sensors.map((sensor) => sensor.id).sort(),
      ['cpu.package.load', 'cpu.package.power', 'cpu.package.temp']
    );
  } finally {
    collector.child.kill();
  }
});

test('sequence numbers only ever grow, across every message it sends', async () => {
  const collector = launch();
  try {
    collector.send('hello');
    collector.send('start', { interval_ms: 250, detail: 'representative' });

    const sequences = [];
    for (let i = 0; i < 6; i += 1) {
      sequences.push((await collector.next()).sequence);
    }
    for (let i = 1; i < sequences.length; i += 1) {
      assert.ok(
        sequences[i] > sequences[i - 1],
        `strictly increasing: ${sequences}`
      );
    }
  } finally {
    collector.child.kill();
  }
});

test('start is confirmed with the effective interval and then samples arrive', async () => {
  const collector = launch();
  try {
    collector.send('hello');
    await collector.next(); // hello_ack
    await collector.next(); // capabilities

    // Below the protocol's 250 ms floor: the collector adjusts it and says so in `started`.
    collector.send('start', { interval_ms: 10, detail: 'per_core' });
    const started = await collector.next();
    assert.equal(started.type, 'started');
    assert.equal(started.payload.interval_ms, 250);
    assert.equal(started.payload.detail, 'per_core');

    const sample = await collector.next();
    assert.equal(sample.type, 'sample');
    assert.equal(sample.session_nonce, NONCE);
    assert.equal(sample.payload.values.length, 3);
    for (const value of sample.payload.values) {
      assert.equal(value.status, 'ok');
      assert.equal(typeof value.number, 'number');
    }
    assert.ok(!Number.isNaN(Date.parse(sample.timestamp_utc)));
  } finally {
    collector.child.kill();
  }
});

test('it stops sampling on stop and exits when the host closes stdin', async () => {
  const collector = launch();
  collector.send('hello');
  await collector.next();
  await collector.next();
  collector.send('start', { interval_ms: 250, detail: 'representative' });
  await collector.next(); // started

  collector.send('stop');
  let message = await collector.next();
  while (message.type === 'sample') message = await collector.next();
  assert.equal(message.type, 'stopped');

  // EOF on stdin is the host's orderly shutdown, same as for the real collector.
  collector.child.stdin.end();
  const [code] = await once(collector.child, 'exit');
  assert.equal(code, 0);
});
