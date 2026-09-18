// Compara las versiones instaladas de las herramientas de agentes con las validadas en
// .agents/meta/agent-capabilities.yaml. No modifica nada. Sale con 2 si hay diferencias.
// Uso: node scripts/agent/detect-capability-changes.mjs
import { execFileSync } from "node:child_process";
import { read } from "./lib.mjs";

const tools = {
  "claude-code": { cmd: "claude", args: ["--version"], re: /(\d+\.\d+\.\d+)/ },
  "codex-cli": { cmd: "codex", args: ["--version"], re: /(\d+\.\d+\.\d+)/ },
  opencode: { cmd: "opencode", args: ["--version"], re: /(\d+\.\d+\.\d+)/ },
};

const yaml = read(".agents/meta/agent-capabilities.yaml");
let diffs = 0;

for (const [key, t] of Object.entries(tools)) {
  const block = yaml.match(new RegExp(`^  ${key}:\\n([\\s\\S]*?)(?=^  [a-z_-]+:|\\Z)`, "m"));
  const validated = block?.[1].match(/validated_version:\s*"([^"]+)"/)?.[1] ?? null;
  let installed = null;
  try {
    // En Windows cada herramienta se instala distinto: .exe (claude, codex) o envoltorio .cmd (opencode).
    const candidates = process.platform === "win32" ? [t.cmd, `${t.cmd}.exe`, `${t.cmd}.cmd`] : [t.cmd];
    let out = null;
    for (const cmd of candidates) {
      try {
        // Node rechaza ejecutar .cmd sin shell (CVE-2024-27980); los argumentos aquí son constantes, no entrada externa.
        const opts = { encoding: "utf8", timeout: 30000, windowsHide: true, stdio: ["ignore", "pipe", "ignore"] };
        out = cmd.endsWith(".cmd")
          ? execFileSync(`${cmd} ${t.args.join(" ")}`, [], { ...opts, shell: true })
          : execFileSync(cmd, t.args, opts);
        break;
      } catch (e) {
        if (e.code !== "ENOENT" && e.code !== "EINVAL") break;
      }
    }
    installed = out?.match(t.re)?.[1] ?? null;
  } catch {
    installed = null;
  }
  const status =
    installed === null ? "NO INSTALADA" : validated === null ? "SIN VALIDAR" : installed === validated ? "SIN CAMBIOS" : "VERSIÓN DISTINTA";
  if (status === "VERSIÓN DISTINTA") diffs++;
  console.log(`${key.padEnd(12)} instalada=${installed ?? "-"} validada=${validated ?? "-"}  ${status}`);
}

if (diffs) {
  console.log(`\n${diffs} herramienta(s) con versión distinta: consulta la documentación de la versión nueva, clasifica el impacto (SIN IMPACTO / REVISIÓN RECOMENDADA / CAMBIO NECESARIO / INCOMPATIBILIDAD) y pide aprobación antes de tocar adaptadores.`);
  process.exit(2);
}
console.log("Versiones coinciden con las validadas.");
