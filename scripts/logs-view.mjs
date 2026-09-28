import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';

// Constitution XVII: developer-facing output is `dd/MM/yyyy HH:mm:ss,SSS` in Europe/Madrid with
// the zone abbreviation, so the two hours around a DST change are never ambiguous.
const formatter = new Intl.DateTimeFormat('es-ES', {
  timeZone: 'Europe/Madrid',
  day: '2-digit',
  month: '2-digit',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  fractionalSecondDigits: 3,
  hourCycle: 'h23',
  timeZoneName: 'short'
});

export function formatHumanTime(timestamp) {
  const epoch =
    typeof timestamp === 'number' ? timestamp : Date.parse(String(timestamp));
  if (Number.isNaN(epoch)) return 'sin fecha';
  const part = Object.fromEntries(
    formatter.formatToParts(epoch).map(({ type, value }) => [type, value])
  );
  return `${part.day}/${part.month}/${part.year} ${part.hour}:${part.minute}:${part.second},${part.fractionalSecond} ${part.timeZoneName}`;
}

export function formatLine(line) {
  let event;
  try {
    event = JSON.parse(line);
  } catch {
    return line;
  }
  if (event === null || typeof event !== 'object') return line;
  const fields =
    event.fields && Object.keys(event.fields).length > 0
      ? ` ${JSON.stringify(event.fields)}`
      : '';
  const session = event.session_id ? ` [${event.session_id}]` : '';
  return [
    formatHumanTime(event.ts),
    String(event.level ?? 'info').toUpperCase(),
    event.component ?? '-',
    event.target ?? '-',
    event.code ?? 'EVENT',
    `${event.msg ?? ''}${session}${fields}`
  ].join(' ');
}

// `throttlewatch.log` is the live file and `throttlewatch.log.N` its rotations (higher N is older);
// `throttlewatch-launcher.log` belongs to the elevated launcher. Oldest first, per base name.
export function orderLogFiles(names) {
  const parsed = names
    .map((name) => /^(throttlewatch[^.]*\.log)(?:\.(\d+))?$/i.exec(name))
    .filter((match) => match !== null)
    .map((match) => ({
      name: match[0],
      base: match[1],
      rotation: match[2] === undefined ? 0 : Number(match[2])
    }));
  parsed.sort((a, b) =>
    a.base === b.base ? b.rotation - a.rotation : a.base.localeCompare(b.base)
  );
  return parsed.map((entry) => entry.name);
}

async function main() {
  const directory =
    process.argv.slice(2).find((argument) => argument !== '--') ?? '';
  if (!directory) {
    process.stdout.write(
      'Indica la carpeta de registros: pnpm logs:view -- <carpeta>\n'
    );
    process.exitCode = 1;
    return;
  }
  for (const name of orderLogFiles(await readdir(directory))) {
    const contents = await readFile(join(directory, name), 'utf8');
    process.stdout.write(`== ${name} ==\n`);
    for (const line of contents.split(/\r?\n/u)) {
      if (line.trim()) process.stdout.write(`${formatLine(line)}\n`);
    }
  }
}

if (import.meta.main) {
  await main();
}
