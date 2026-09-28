import assert from 'node:assert/strict';
import { test } from 'node:test';
import { formatHumanTime, formatLine, orderLogFiles } from './logs-view.mjs';

test('the spring-forward instant keeps the right hour and zone on each side', () => {
  assert.equal(
    formatHumanTime('2026-03-29T00:59:59.999Z'),
    '29/03/2026 01:59:59,999 CET'
  );
  assert.equal(
    formatHumanTime('2026-03-29T01:00:00.000Z'),
    '29/03/2026 03:00:00,000 CEST'
  );
});

test('the fall-back instant tells the repeated hour apart by its zone', () => {
  assert.equal(
    formatHumanTime('2026-10-25T00:59:59.999Z'),
    '25/10/2026 02:59:59,999 CEST'
  );
  assert.equal(
    formatHumanTime('2026-10-25T01:00:00.000Z'),
    '25/10/2026 02:00:00,000 CET'
  );
});

test('an unparseable timestamp is labelled, never a raw number', () => {
  assert.equal(formatHumanTime('not a date'), 'sin fecha');
});

test('a log line shows time, level, component, target, code and message', () => {
  const line = JSON.stringify({
    ts: '2026-09-28T05:09:22.923Z',
    level: 'WARN',
    component: 'guided',
    target: 'throttlewatch_lib::commands',
    code: 'GUIDED_SESSION_ENDED',
    msg: 'guided session ended',
    session_id: 'guided-1',
    fields: { reason: 'guided.parent_missing' }
  });
  assert.equal(
    formatLine(line),
    '28/09/2026 07:09:22,923 CEST WARN guided throttlewatch_lib::commands GUIDED_SESSION_ENDED ' +
      'guided session ended [guided-1] {"reason":"guided.parent_missing"}'
  );
});

test('a line that is not JSON is shown untouched', () => {
  assert.equal(formatLine('plain text'), 'plain text');
});

test('rotated files come before the live one and the launcher log is included', () => {
  assert.deepEqual(
    orderLogFiles([
      'throttlewatch.log',
      'throttlewatch.log.1',
      'other.txt',
      'throttlewatch.log.2',
      'throttlewatch-launcher.log'
    ]),
    [
      'throttlewatch-launcher.log',
      'throttlewatch.log.2',
      'throttlewatch.log.1',
      'throttlewatch.log'
    ]
  );
});
