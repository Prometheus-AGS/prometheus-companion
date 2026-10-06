// Read-only renderer. Values are text, never sed/shell expressions.
import fs from 'node:fs';
import path from 'node:path';
const [template, binary, logDirectory, home, packRoot, environmentJson] = process.argv.slice(2);
const xml = (value) => {
  if (typeof value !== 'string' || /[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value)) throw new Error('invalid XML path value');
  return value.replace(/[&<>"'\r\n\t]/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;', '\r': '&#13;', '\n': '&#10;', '\t': '&#9;' }[character]));
};
try {
  const environment = JSON.parse(environmentJson);
  const allowed = new Set(['PROMETHEUS_DEVICE_KEY_FILE', 'PROMETHEUS_DATA_DIR', 'PROMETHEUS_SKILLS_DIR', 'SOVEREIGN_SYNC_SOCKET', 'SOVEREIGN_SYNC_CONFIG', 'XDG_CONFIG_HOME', 'CLAUDE_CONFIG_DIR']);
  const values = { __COMPANION_BIN__: binary, __LOG_DIR__: logDirectory, __HOME__: home, __WORKING_DIRECTORY__: home, __PROMETHEUS_PACK_ROOT__: packRoot };
  for (const value of [...Object.values(values), ...Object.values(environment)]) {
    if (typeof value !== 'string' || (value && !path.isAbsolute(value))) throw new Error('service paths must be absolute');
  }
  const replacements = Object.fromEntries(Object.entries(values).map(([key, value]) => [key, xml(value)]));
  replacements.__OPTIONAL_ENVIRONMENT__ = Object.entries(environment).filter(([, value]) => value !== '').map(([key, value]) => {
    if (!allowed.has(key)) throw new Error('unsupported service environment key');
    return `    <key>${key}</key>\n    <string>${xml(value)}</string>`;
  }).join('\n');
  const output = fs.readFileSync(template, 'utf8').replace(/__[A-Z_]+__/g, (token) => {
    if (!Object.hasOwn(replacements, token)) throw new Error('unknown plist template token');
    return replacements[token];
  });
  process.stdout.write(output);
} catch (error) { process.stderr.write(`render-companion-plist: ${error.message}\n`); process.exitCode = 1; }
