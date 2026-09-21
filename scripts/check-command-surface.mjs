import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const tauriSource = await readFile(
  resolve(root, 'apps/desktop/src-tauri/src/lib.rs'),
  'utf8'
);
const commandSource = await readFile(
  resolve(root, 'apps/desktop/src-tauri/src/commands/mod.rs'),
  'utf8'
);
const bridgeSource = await readFile(
  resolve(root, 'apps/desktop/src/lib/bridge/index.ts'),
  'utf8'
);
const bridgeSchemas = await readFile(
  resolve(root, 'apps/desktop/src/lib/bridge/schemas.ts'),
  'utf8'
);
const contract = await readFile(
  resolve(root, 'specs/001-cpu-thermal-diagnostics/contracts/application-commands.md'),
  'utf8'
);
const capabilities = JSON.parse(
  await readFile(
    resolve(root, 'apps/desktop/src-tauri/capabilities/default.json'),
    'utf8'
  )
);

const handlerBlock = tauriSource.match(
  /tauri::generate_handler!\[([\s\S]*?)\]\)/
);
if (!handlerBlock) {
  throw new Error('No se encontró el registro de comandos Tauri');
}

const registered = [...handlerBlock[1].matchAll(/(?:commands|updates_app)::([a-z][a-z0-9_]*)/g)].map(
  ([, name]) => name
);
const bridged = [...bridgeSource.matchAll(/case '([a-z][a-z0-9_]*)':/g)].map(
  ([, name]) => name
);
const argsBlock = bridgeSchemas.match(
  /export const commandArgsSchemas = \{([\s\S]*?)\n\} as const;/
);
const schemaKeys = argsBlock
  ? [...argsBlock[1].matchAll(/^\s{2}([a-z][a-z0-9_]*):/gm)].map(([, name]) => name)
  : [];

const missing = (left, right) => left.filter((name) => !right.includes(name));
const errors = [];
for (const name of missing(registered, bridged)) {
  errors.push(`comando registrado sin caso en el puente: ${name}`);
}
for (const name of missing(registered, schemaKeys)) {
  errors.push(`comando registrado sin esquema de argumentos: ${name}`);
}
for (const name of registered) {
  if (!new RegExp(`\\b${name}\\s*\\(`).test(contract)) {
    errors.push(`comando registrado ausente del contrato: ${name}`);
  }
}

const pluginPermissions = capabilities.permissions.filter(
  (permission) => !permission.startsWith('core:')
);
if (pluginPermissions.length > 0) {
  errors.push(`capabilities contiene permisos de plugin: ${pluginPermissions.join(', ')}`);
}
if (tauriSource.includes('open_external_url(url:') || bridgeSource.includes('open_external_url(url:')) {
  errors.push('la superficie de URL externa no puede aceptar una URL libre');
}
for (const command of ['delete_monitoring_data', 'reset_application', 'delete_session']) {
  const start = commandSource.indexOf(`pub fn ${command}`);
  const nextCommand = commandSource.indexOf('\n#[tauri::command]', start + 1);
  const commandBody = start >= 0
    ? commandSource.slice(start, nextCommand >= 0 ? nextCommand : start + 2_000)
    : '';
  if (!commandBody.includes('confirmation_token') && !commandBody.includes('ConfirmationRequest')) {
    errors.push(`comando destructivo sin token de confirmación: ${command}`);
  }
}
if (!commandSource.includes('prepare_for_wipe')) {
  errors.push('no se encontró la guardia de estado activo para operaciones destructivas');
}
if (!commandSource.includes('access_not_installable')) {
  errors.push('no se encontró la guardia de estado para acceso avanzado');
}

if (errors.length > 0) {
  console.error(errors.map((error) => `✗ ${error}`).join('\n'));
  process.exitCode = 1;
}

console.log(
  `command-surface: ${registered.length} comandos registrados, ` +
    `${pluginPermissions.length} permisos de plugin, ${errors.length} errores`
);
