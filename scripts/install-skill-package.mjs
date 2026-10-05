// Companion-owned registration. CLI arguments are arrays; manifest/config
// values never become shell source. Dry-run makes no directories/locks or calls.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const owner = 'prometheus-companion';
const options = { harness: 'claude-code', dryRun: false, bin: path.join(os.homedir(), '.local/bin/sovereign-sync'), claudeBin: 'claude' };
const usage = 'Usage: bash scripts/install-skill-package.sh [--harness claude-code] [--bin ABSOLUTE-PATH] [--claude-bin PATH] [--dry-run]';
const object = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const canonical = (value) => JSON.stringify(value, (_key, item) => object(item) ? Object.fromEntries(Object.keys(item).sort().map((key) => [key, item[key]])) : item);
const digest = (value) => value === undefined ? null : createHash('sha256').update(canonical(value)).digest('hex');
const info = (text) => process.stdout.write(`install-skill-package: ${text}\n`);

function stat(file) {
  try { return fs.lstatSync(file); } catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}
function safeAncestors(file) {
  for (let current = path.resolve(file); ; current = path.dirname(current)) {
    const entry = stat(current);
    if (entry?.isSymbolicLink()) throw new Error(`refusing symlink path ${current}`);
    if (current === path.dirname(current)) break;
  }
}
function readJson(file, fallback) {
  const entry = stat(file);
  if (!entry) return fallback;
  if (!entry.isFile() || entry.isSymbolicLink()) throw new Error(`not a regular file: ${file}`);
  let value;
  try { value = JSON.parse(fs.readFileSync(file, 'utf8')); }
  catch { throw new Error(`malformed JSON in ${file}; original preserved`); }
  if (!object(value)) throw new Error(`expected JSON object: ${file}`);
  return value;
}
function syncDirectory(directory) {
  const fd = fs.openSync(directory, 'r');
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}
function writePrivate(file, value) {
  const fd = fs.openSync(file, 'wx', 0o600);
  try { fs.writeFileSync(fd, value); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  syncDirectory(path.dirname(file));
}
function storeReceipt(file, receipt) {
  const temporary = `${file}.${randomUUID()}.tmp`;
  writePrivate(temporary, `${JSON.stringify(receipt, null, 2)}\n`);
  fs.renameSync(temporary, file);
  syncDirectory(path.dirname(file));
}
function userServers(file) {
  const config = readJson(file, {});
  if (config.mcpServers !== undefined && !object(config.mcpServers)) throw new Error(`invalid user mcpServers in ${file}`);
  return config.mcpServers ?? {};
}
function cli(args, env) {
  const result = spawnSync(options.claudeBin, ['mcp', ...args], { env, cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  // CLI diagnostics/configs can contain credentials; do not echo captured data.
  if (result.error || result.status !== 0) throw new Error(`Claude MCP CLI failed (${result.error?.code ?? result.status}); registration is incomplete; retained private receipt/backup must be reconciled`);
}

function main() {
  const args = process.argv.slice(2);
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--dry-run') options.dryRun = true;
    else if (arg === '--help' || arg === '-h') { info(usage); return; }
    else if (['--harness', '--bin', '--claude-bin'].includes(arg)) {
      const value = args[++index];
      if (!value || value.startsWith('--')) throw new Error(`${arg} requires a value`);
      options[{ '--harness': 'harness', '--bin': 'bin', '--claude-bin': 'claudeBin' }[arg]] = value;
    } else throw new Error(`unknown option: ${arg}`);
  }
  if (options.harness !== 'claude-code') throw new Error('only claude-code is currently supported');
  if (!['darwin', 'linux'].includes(process.platform)) throw new Error('this installer currently requires a POSIX host; native Windows registration is not implemented');
  if (!path.isAbsolute(options.bin)) throw new Error('--bin must be an absolute installed sovereign-sync path');
  if (options.claudeBin.includes(path.sep)) options.claudeBin = path.resolve(options.claudeBin);
  const configRoot = path.resolve(process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude'));
  const userConfig = process.env.CLAUDE_CONFIG_DIR ? path.join(configRoot, '.claude.json') : path.join(os.homedir(), '.claude.json');
  const skillsRoot = path.join(configRoot, 'skills');
  const receiptDirectory = path.join(configRoot, '.prometheus-companion');
  const receiptFile = path.join(receiptDirectory, 'skill-package-receipt.json');
  safeAncestors(configRoot);
  safeAncestors(userConfig);
  safeAncestors(skillsRoot);
  safeAncestors(receiptDirectory);
  const declaration = readJson(path.join(root, 'skill-package.json'));
  if (declaration?.name !== owner) throw new Error('unexpected skill-package owner');
  const receipt = readJson(receiptFile, { schemaVersion: 1, owner, servers: {}, skills: {} });
  if (receipt.schemaVersion !== 1 || receipt.owner !== owner || !object(receipt.servers) || !object(receipt.skills)) throw new Error('unsupported ownership receipt; preserve and resolve before installing');
  const plannedReceiptHash = digest(receipt);
  const servers = userServers(userConfig);
  if (!object(declaration.mcpServers ?? {})) throw new Error('invalid declared mcpServers');
  const registrations = Object.entries(declaration.mcpServers ?? {}).map(([name, spec]) => {
    if (!/^[A-Za-z0-9_-]+$/.test(name) || !object(spec) || !object(spec.env ?? {}) || (spec.type !== undefined && spec.type !== 'stdio')) throw new Error('only named stdio MCP declarations are supported');
    const overrides = {};
    for (const key of ['PROMETHEUS_DATA_DIR', 'PROMETHEUS_DEVICE_KEY_FILE', 'PROMETHEUS_PACK_ROOT', 'SOVEREIGN_SYNC_SOCKET', 'SOVEREIGN_SYNC_CONFIG', 'XDG_CONFIG_HOME', 'CLAUDE_CONFIG_DIR']) {
      if (process.env[key]) {
        if (!path.isAbsolute(process.env[key])) throw new Error(`explicit ${key} must be an absolute path`);
        overrides[key] = process.env[key];
      }
    }
    const entry = { ...spec, type: 'stdio', command: options.bin, env: { ...(spec.env ?? {}), ...overrides, PROMETHEUS_SKILLS_DIR: skillsRoot } };
    if (!Array.isArray(entry.args) || !entry.args.every((arg) => typeof arg === 'string') || !object(entry.env) || !Object.values(entry.env).every((value) => typeof value === 'string')) throw new Error('invalid stdio args/environment');
    const previous = servers[name];
    const record = receipt.servers[name];
    const currentHash = digest(previous);
    const desiredHash = digest(entry);
    const pending = record?.pending;
    const owned = record?.owned === true && (record.currentHash === currentHash || pending?.desiredHash === currentHash || (currentHash === null && pending));
    if (previous !== undefined && currentHash !== desiredHash && !owned) throw new Error(`preserving unowned/edited user MCP entry ${name}; no automatic overwrite`);
    if (pending && pending.desiredHash !== desiredHash) throw new Error(`unfinished ${name} registration targets another artifact/configuration; reconcile its receipt first`);
    return { name, entry, previous, currentHash, desiredHash, owned };
  });
  const skillDirectory = path.resolve(root, declaration.skills || 'skills');
  if (!skillDirectory.startsWith(root + path.sep)) throw new Error('declared skills must be inside this repository');
  safeAncestors(skillDirectory);
  const links = fs.readdirSync(skillDirectory, { withFileTypes: true }).filter((entry) => entry.isDirectory() && stat(path.join(skillDirectory, entry.name, 'SKILL.md'))?.isFile()).map((entry) => {
    const source = path.join(skillDirectory, entry.name);
    const target = path.join(skillsRoot, entry.name);
    const previous = stat(target);
    if (previous && !previous.isSymbolicLink()) throw new Error(`preserving non-symlink skill ${target}; no automatic overwrite`);
    const current = previous?.isSymbolicLink() ? path.resolve(path.dirname(target), fs.readlinkSync(target)) : null;
    const record = receipt.skills[entry.name];
    const owned = record?.source === current || (record?.pending && record.previousSource === current);
    if (record?.pending && record.source !== source) throw new Error(`unfinished skill ${entry.name} targets another source; reconcile its receipt first`);
    if (previous && current !== source && !owned) throw new Error(`preserving unowned/edited skill ${target}; no automatic overwrite`);
    return { name: entry.name, source, target, current, owned };
  });
  info(`harness=claude-code scope=user config=${userConfig} dry_run=${options.dryRun}`);
  for (const item of registrations) info(`${item.name}: ${item.currentHash === item.desiredHash ? 'preserve matching registration' : item.previous ? 'replace receipt-owned registration' : 'register'}; binary=${options.bin}`);
  for (const item of links) info(`${item.name}: ${item.current === item.source ? 'preserve matching skill link' : 'link'} ${item.target}`);
  if (options.dryRun) return;
  fs.accessSync(options.bin, fs.constants.X_OK);
  fs.mkdirSync(receiptDirectory, { recursive: true, mode: 0o700 });
  if ((fs.statSync(receiptDirectory).mode & 0o077) !== 0) throw new Error('ownership/backup directory must be private; original preserved');
  const lockPath = path.join(receiptDirectory, 'install.lock');
  const lock = fs.openSync(lockPath, 'wx', 0o600);
  try {
    if (digest(readJson(receiptFile, { schemaVersion: 1, owner, servers: {}, skills: {} })) !== plannedReceiptHash) throw new Error('ownership receipt changed since planning; retry without overwriting it');
    const env = { ...process.env, ...(process.env.CLAUDE_CONFIG_DIR ? { CLAUDE_CONFIG_DIR: configRoot } : {}) };
    for (const item of registrations) {
      const before = userServers(userConfig);
      if (digest(before[item.name]) !== item.currentHash) throw new Error(`user registration ${item.name} changed since planning; original preserved`);
      if (item.currentHash === item.desiredHash) {
        if (item.owned) {
          const prior = receipt.servers[item.name];
          receipt.servers[item.name] = { ...prior, owned: true, currentHash: item.desiredHash,
            backupPath: prior.pending?.backupPath ?? prior.backupPath };
          delete receipt.servers[item.name].pending;
          storeReceipt(receiptFile, receipt);
        }
        else info(`${item.name}: existing matching entry retained without claiming ownership`);
        continue;
      }
      const previousRecord = receipt.servers[item.name];
      const backups = [...(previousRecord?.backups ?? []), previousRecord?.backupPath, previousRecord?.pending?.backupPath].filter((value) => typeof value === 'string');
      const backupPath = stat(userConfig) ? path.join(receiptDirectory, `user-config-${randomUUID()}.json`) : null;
      if (backupPath) writePrivate(backupPath, fs.readFileSync(userConfig));
      receipt.servers[item.name] = { owned: true, currentHash: item.currentHash, backups: [...new Set(backups)], pending: { desiredHash: item.desiredHash, previousHash: item.currentHash, backupPath } };
      storeReceipt(receiptFile, receipt); // Durable intent precedes remove/add.
      try {
        if (item.previous !== undefined) cli(['remove', item.name, '--scope', 'user'], env);
        cli(['add-json', item.name, JSON.stringify(item.entry), '--scope', 'user'], env);
      } catch (error) {
        const current = userServers(userConfig)[item.name];
        // Restore only our removed entry; never overwrite an unexpected writer
        // or restore the whole config over unrelated changes.
        if (current === undefined && item.previous !== undefined) {
          try { cli(['add-json', item.name, JSON.stringify(item.previous), '--scope', 'user'], env); }
          catch { /* Keep the pending receipt and exact private backup. */ }
        }
        throw error;
      }
      const after = userServers(userConfig);
      if (digest(after[item.name]) !== item.desiredHash) throw new Error(`CLI did not install expected ${item.name}; private backup and pending receipt retained`);
      for (const [name, entry] of Object.entries(before)) {
        if (name !== item.name && digest(after[name]) !== digest(entry)) throw new Error('unrelated MCP configuration changed; private backup retained for owner reconciliation');
      }
      receipt.servers[item.name] = { owned: true, currentHash: item.desiredHash, backupPath, backups: [...new Set(backups)] };
      storeReceipt(receiptFile, receipt);
      info(`${item.name}: registered through Claude CLI user scope`);
    }
    fs.mkdirSync(skillsRoot, { recursive: true });
    for (const item of links) {
      const current = stat(item.target);
      if (current && !current.isSymbolicLink()) throw new Error(`skill became non-symlink: ${item.target}; original preserved`);
      const currentSource = current?.isSymbolicLink() ? path.resolve(path.dirname(item.target), fs.readlinkSync(item.target)) : null;
      if (currentSource === item.source) {
        if (!item.owned) info(`${item.name}: matching unowned link preserved`);
        else { receipt.skills[item.name] = { source: item.source }; storeReceipt(receiptFile, receipt); }
        continue;
      }
      if (currentSource !== item.current) throw new Error(`skill changed since planning: ${item.target}`);
      const temporary = path.join(skillsRoot, `.companion-${randomUUID()}`);
      fs.symlinkSync(item.source, temporary, 'dir');
      receipt.skills[item.name] = { source: item.source, previousSource: item.current, temporary, pending: true };
      storeReceipt(receiptFile, receipt);
      fs.renameSync(temporary, item.target);
      syncDirectory(skillsRoot);
      receipt.skills[item.name] = { source: item.source };
      storeReceipt(receiptFile, receipt);
    }
    info('owned registration complete; reload Claude Code; connectivity/operation acceptance is separate');
  } finally {
    const identity = fs.fstatSync(lock);
    fs.closeSync(lock);
    const current = stat(lockPath);
    if (current?.ino === identity.ino && current?.dev === identity.dev) fs.unlinkSync(lockPath);
  }
}

try { main(); } catch (error) { process.stderr.write(`install-skill-package: ${error.message}\n`); process.exitCode = 1; }
