// Utilidades compartidas por los scripts del sistema de agentes. Node ≥ 20, sin dependencias.
import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
export const GENERATED_BANNER =
  "<!-- GENERADO por scripts/agent/sync-adapters.mjs a partir de {source}. No editar: cambia la fuente y vuelve a sincronizar. -->\n\n";

export function read(rel) {
  return readFileSync(join(ROOT, rel), "utf8");
}

export function exists(rel) {
  return existsSync(join(ROOT, rel));
}

export function listDirs(rel) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) return [];
  return readdirSync(abs).filter((n) => statSync(join(abs, n)).isDirectory()).sort();
}

export function listMd(rel) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) return [];
  return readdirSync(abs).filter((n) => n.endsWith(".md")).sort();
}

// Frontmatter YAML mínimo: escalares y listas de cadenas de un nivel.
export function parseFrontmatter(text) {
  const m = text.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);
  if (!m) return { data: {}, body: text, raw: "" };
  const data = {};
  let key = null;
  for (const line of m[1].split(/\r?\n/)) {
    const list = line.match(/^\s+-\s+(.*)$/);
    if (list && key) {
      (data[key] ??= []).push(unquote(list[1]));
      continue;
    }
    const kv = line.match(/^([A-Za-z_][\w-]*):\s*(.*)$/);
    if (kv) {
      key = kv[1];
      data[key] = kv[2] === "" ? [] : unquote(kv[2]);
    }
  }
  return { data, body: m[2], raw: m[1] };
}

function unquote(s) {
  const t = s.trim();
  return /^".*"$/.test(t) || /^'.*'$/.test(t) ? t.slice(1, -1) : t;
}

export function tomlString(s) {
  return JSON.stringify(s);
}

// Cadena TOML multilínea literal; se evita la secuencia ''' dentro del texto.
export function tomlMultiline(s) {
  return `'''\n${s.replaceAll("'''", "''\\'")}'''`;
}

// Adaptadores que produce sync-adapters: { rutaRelativa: contenido }.
export function renderAdapters() {
  const out = {};

  // Reglas de Claude: copia con cabecera de generado; el frontmatter `paths` es compatible.
  for (const f of listMd(".agents/rules")) {
    const src = `.agents/rules/${f}`;
    const text = read(src);
    const { raw, body } = parseFrontmatter(text);
    const fm = raw ? `---\n${raw}\n---\n\n` : "";
    out[`.claude/rules/${f}`] = fm + GENERATED_BANNER.replace("{source}", src) + body.replace(/^\s+/, "");
  }

  // Skills propias: Claude no lee .agents/skills, así que se copian.
  for (const d of listDirs(".agents/skills")) {
    if (d.startsWith("speckit-")) continue; // propiedad de Spec Kit
    const src = `.agents/skills/${d}/SKILL.md`;
    const text = read(src);
    const { raw, body } = parseFrontmatter(text);
    out[`.claude/skills/${d}/SKILL.md`] =
      `---\n${raw}\n---\n\n` + GENERATED_BANNER.replace("{source}", src) + body.replace(/^\s+/, "");
  }

  // Roles → subagente de Claude (solo lectura) y agente de Codex (sandbox read-only).
  for (const f of listMd(".agents/roles")) {
    const src = `.agents/roles/${f}`;
    const { data, body } = parseFrontmatter(read(src));
    const name = data.name;
    const description = data.description;
    const instructions = body.replace(/^\s+/, "").trimEnd() + "\n";

    out[`.claude/agents/${name}.md`] =
      `---\nname: ${name}\ndescription: ${yamlQuote(description)}\ntools: Read, Glob, Grep, Bash\n` +
      `disallowedTools: Write, Edit, NotebookEdit, WebFetch\npermissionMode: plan\nmodel: inherit\n---\n\n` +
      GENERATED_BANNER.replace("{source}", src) +
      "Nota Claude Code: `permissionMode: plan` impide editar; `Bash` queda para comandos de solo lectura (`git diff`, `git log`). No ejecutes nada que escriba ficheros.\n\n" +
      instructions;

    out[`.codex/agents/${name}.toml`] =
      `# GENERADO por scripts/agent/sync-adapters.mjs a partir de ${src}. No editar.\n` +
      `name = ${tomlString(name)}\n` +
      `description = ${tomlString(description)}\n` +
      `sandbox_mode = "read-only"\n` +
      `developer_instructions = ${tomlMultiline(instructions)}\n`;
  }

  return out;
}

function yamlQuote(s) {
  return JSON.stringify(s);
}
