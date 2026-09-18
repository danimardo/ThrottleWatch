// Regenera los adaptadores de Claude Code y Codex a partir de la fuente canónica (.agents/).
// Uso: node scripts/agent/sync-adapters.mjs [--dry-run]
import { writeFileSync, mkdirSync, readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { ROOT, renderAdapters } from "./lib.mjs";

const dry = process.argv.includes("--dry-run");
const adapters = renderAdapters();
let changed = 0;

for (const [rel, content] of Object.entries(adapters)) {
  const abs = join(ROOT, rel);
  const current = existsSync(abs) ? readFileSync(abs, "utf8") : null;
  if (current === content) continue;
  changed++;
  console.log(`${current === null ? "crear    " : "actualizar"} ${rel}`);
  if (!dry) {
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, content, "utf8");
  }
}

console.log(changed === 0 ? "Adaptadores al día." : `${changed} adaptador(es) ${dry ? "pendiente(s)" : "escrito(s)"}.`);
