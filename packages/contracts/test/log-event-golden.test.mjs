// Constitution XVII: every line the backend writes must satisfy `log-event.schema.json`. The Rust
// end-to-end test compares its real log file with this golden file; this test checks the golden
// file against the schema, so the two together cover the path from `log_*!` to the contract.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const read = (relative) => readFileSync(fileURLToPath(new URL(relative, import.meta.url)), 'utf8');
const schema = JSON.parse(read('../schemas/log-event.schema.json'));
const golden = read('../../trace-fixtures/logs/log-events.golden.jsonl')
  .split(/\r?\n/)
  .filter((line) => line.trim() !== '');

// The subset of JSON Schema that `log-event.schema.json` uses; no validator is part of the pinned
// stack, and pulling one in for a single schema would need a constitution amendment.
function validate(value, rule, path = '$') {
  const errors = [];
  const types = rule.type === undefined ? [] : [rule.type].flat();
  const typeOf = (item) =>
    item === null ? 'null' : Array.isArray(item) ? 'array' : Number.isInteger(item) ? 'integer' : typeof item;
  if (types.length > 0 && !types.some((type) => type === typeOf(value) || (type === 'number' && typeof value === 'number'))) {
    return [`${path}: tipo ${typeOf(value)}, se esperaba ${types.join('|')}`];
  }
  if (rule.enum && !rule.enum.includes(value)) errors.push(`${path}: ${JSON.stringify(value)} fuera de ${rule.enum.join('|')}`);
  if (typeof value === 'string') {
    if (rule.minLength !== undefined && value.length < rule.minLength) errors.push(`${path}: demasiado corto`);
    if (rule.maxLength !== undefined && value.length > rule.maxLength) errors.push(`${path}: demasiado largo`);
    if (rule.pattern && !new RegExp(rule.pattern, 'u').test(value)) errors.push(`${path}: no cumple ${rule.pattern}`);
    if (rule.format === 'date-time' && (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/.test(value) || Number.isNaN(Date.parse(value)))) {
      errors.push(`${path}: no es date-time`);
    }
  }
  if (typeof value === 'number' && rule.minimum !== undefined && value < rule.minimum) errors.push(`${path}: menor que ${rule.minimum}`);
  if (typeOf(value) === 'object') {
    for (const key of rule.required ?? []) if (!(key in value)) errors.push(`${path}: falta ${key}`);
    for (const [key, item] of Object.entries(value)) {
      const property = rule.properties?.[key];
      if (property) errors.push(...validate(item, property, `${path}.${key}`));
      else if (rule.additionalProperties === false) errors.push(`${path}: propiedad no permitida ${key}`);
    }
  }
  return errors;
}

test('the golden log file is not empty', () => {
  assert.ok(golden.length > 0);
});

test('every golden log line satisfies log-event.schema.json', () => {
  for (const [index, line] of golden.entries()) {
    assert.deepEqual(validate(JSON.parse(line), schema), [], `línea ${index + 1}: ${line}`);
  }
});

test('the validator rejects what the schema forbids', () => {
  const valid = JSON.parse(golden[0]);
  assert.notDeepEqual(validate({ ...valid, code: 'lower_case' }, schema), []);
  assert.notDeepEqual(validate({ ...valid, component: 'somewhere' }, schema), []);
  assert.notDeepEqual(validate({ ...valid, extra: 1 }, schema), []);
  const { msg: _msg, ...withoutMessage } = valid;
  assert.notDeepEqual(validate(withoutMessage, schema), []);
});
