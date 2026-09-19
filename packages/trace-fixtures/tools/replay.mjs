import { readFile } from 'node:fs/promises';

const args = process.argv.slice(2);
const traceIndex = args.indexOf('--trace');
const tracePath = traceIndex >= 0 ? args[traceIndex + 1] : undefined;

if (!tracePath) {
  process.stderr.write('Missing required --trace path.\n');
  process.exitCode = 2;
} else {
  try {
    const raw = await readFile(tracePath, 'utf8');
    const parsed = JSON.parse(raw);
    const messages = Array.isArray(parsed) ? parsed : parsed.messages;

    if (!Array.isArray(messages) || messages.length === 0) {
      throw new Error('Trace must contain a non-empty messages array.');
    }

    messages.forEach((message, index) => {
      validateEnvelope(message, index);
      process.stdout.write(`${JSON.stringify(message)}\n`);
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : 'Unknown replay error.';
    process.stderr.write(`${message}\n`);
    process.exitCode = 1;
  }
}

function validateEnvelope(message, index) {
  if (!message || typeof message !== 'object' || Array.isArray(message)) {
    throw new Error(`Trace message ${index} is not an object.`);
  }

  const required = [
    'protocol_version',
    'session_nonce',
    'sequence',
    'timestamp_utc',
    'type',
    'payload'
  ];
  const missing = required.filter((key) => !(key in message));
  if (missing.length > 0) {
    throw new Error(
      `Trace message ${index} is missing: ${missing.join(', ')}.`
    );
  }

  if (
    message.protocol_version !== 1 ||
    typeof message.session_nonce !== 'string' ||
    message.sequence < 0
  ) {
    throw new Error(`Trace message ${index} has an invalid envelope.`);
  }
}
