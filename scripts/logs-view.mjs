import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';

const directory = process.argv.slice(2).find((argument) => argument !== '--') ?? '';
if (!directory) {
  process.stdout.write(
    'Indica la carpeta de registros: pnpm logs:view -- <carpeta>\n'
  );
  process.exitCode = 1;
} else {
  const formatter = new Intl.DateTimeFormat('es-ES', {
    timeZone: 'Europe/Madrid',
    dateStyle: 'short',
    timeStyle: 'medium'
  });
  const names = (await readdir(directory))
    .filter((name) => /^throttlewatch.*\.log$/i.test(name))
    .sort();
  for (const name of names) {
    const contents = await readFile(join(directory, name), 'utf8');
    for (const line of contents.split(/\r?\n/u)) {
      if (!line.trim()) continue;
      try {
        const event = JSON.parse(line);
        const timestamp =
          typeof event.ts === 'string' ? Date.parse(event.ts) : Number.NaN;
        const when = Number.isNaN(timestamp)
          ? 'sin fecha'
          : formatter.format(timestamp);
        process.stdout.write(
          `${when} ${event.level ?? 'info'} ${event.code ?? 'EVENT'} ${event.msg ?? ''}\n`
        );
      } catch {
        process.stdout.write(`${line}\n`);
      }
    }
  }
}
