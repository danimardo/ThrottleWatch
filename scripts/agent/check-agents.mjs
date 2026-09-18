// Comprueba la coherencia del sistema de agentes. Sale con código 1 si hay errores.
// Uso: node scripts/agent/check-agents.mjs
import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { ROOT, read, exists, listDirs, listMd, parseFrontmatter, renderAdapters } from "./lib.mjs";

const errors = [];
const warnings = [];
const MAX_AGENTS_LINES = 200;

// 1. AGENTS.md
if (!exists("AGENTS.md")) errors.push("Falta AGENTS.md en la raíz.");
else {
  const lines = read("AGENTS.md").split(/\r?\n/).length;
  if (lines > MAX_AGENTS_LINES) errors.push(`AGENTS.md tiene ${lines} líneas (máximo ${MAX_AGENTS_LINES}).`);
  const bytes = Buffer.byteLength(read("AGENTS.md"), "utf8");
  if (bytes > 32 * 1024) errors.push(`AGENTS.md pesa ${bytes} bytes; Codex lo trunca a 32 KiB por defecto.`);
}

// 2. CLAUDE.md importa AGENTS.md
if (!exists("CLAUDE.md")) errors.push("Falta CLAUDE.md (adaptador de Claude Code).");
else if (!/^@AGENTS\.md\s*$/m.test(read("CLAUDE.md"))) errors.push("CLAUDE.md no importa `@AGENTS.md` en una línea propia.");

// 3. Frontmatter de reglas, skills y roles canónicos
for (const f of listMd(".agents/rules")) {
  const { data } = parseFrontmatter(read(`.agents/rules/${f}`));
  if (!Array.isArray(data.paths) || data.paths.length === 0) errors.push(`.agents/rules/${f}: falta \`paths\` con al menos un glob.`);
}
for (const d of listDirs(".agents/skills")) {
  const p = `.agents/skills/${d}/SKILL.md`;
  if (!exists(p)) { errors.push(`${p} no existe.`); continue; }
  const { data } = parseFrontmatter(read(p));
  const name = String(data.name ?? "").replace(/^"|"$/g, "");
  if (name !== d) errors.push(`${p}: \`name\` (${name || "vacío"}) debe coincidir con el directorio (${d}).`);
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(d) || d.length > 64) errors.push(`${p}: nombre inválido para Codex/OpenCode.`);
  const desc = String(data.description ?? "");
  if (desc.length < 1 || desc.length > 1024) errors.push(`${p}: \`description\` debe tener entre 1 y 1024 caracteres.`);
}
for (const f of listMd(".agents/roles")) {
  const { data } = parseFrontmatter(read(`.agents/roles/${f}`));
  if (!data.name || !data.description) errors.push(`.agents/roles/${f}: faltan \`name\` o \`description\`.`);
  if (data.name && `${data.name}.md` !== f) errors.push(`.agents/roles/${f}: \`name\` debe coincidir con el nombre del fichero.`);
}

// 4. Adaptadores generados coinciden con la fuente
for (const [rel, expected] of Object.entries(renderAdapters())) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) { errors.push(`Adaptador ausente: ${rel} (ejecuta sync-adapters).`); continue; }
  if (readFileSync(abs, "utf8") !== expected) errors.push(`Adaptador divergente: ${rel} (no editar a mano; ejecuta sync-adapters).`);
}

// 5. Skills de Spec Kit presentes para cada integración instalada
if (exists(".specify/integration.json")) {
  const integ = JSON.parse(read(".specify/integration.json"));
  for (const key of integ.installed_integrations ?? []) {
    const manifest = `.specify/integrations/${key}.manifest.json`;
    if (!exists(manifest)) { warnings.push(`Integración ${key} sin manifiesto ${manifest}.`); continue; }
    for (const rel of Object.keys(JSON.parse(read(manifest)).files ?? {})) {
      if (!exists(rel)) errors.push(`Fichero de Spec Kit (${key}) ausente: ${rel}. Reinstala con \`specify integration upgrade ${key}\`.`);
    }
  }
}

// 6. Permisos mínimos de Claude
if (exists(".claude/settings.json")) {
  const s = JSON.parse(read(".claude/settings.json"));
  const deny = s.permissions?.deny ?? [];
  if (!deny.some((d) => d.includes(".env"))) warnings.push(".claude/settings.json: no deniega la lectura de .env*.");
} else warnings.push("Falta .claude/settings.json (permisos del proyecto).");

for (const w of warnings) console.log(`AVISO  ${w}`);
for (const e of errors) console.log(`ERROR  ${e}`);
console.log(errors.length ? `${errors.length} error(es), ${warnings.length} aviso(s).` : `Sistema de agentes coherente (${warnings.length} aviso(s)).`);
process.exit(errors.length ? 1 : 0);
