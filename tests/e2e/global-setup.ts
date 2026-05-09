import { spawn, ChildProcess, execSync } from 'node:child_process';
import { existsSync, rmSync, writeFileSync, mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const ROOT = resolve(__dirname, '../..');
const STATE_FILE = resolve(ROOT, '.e2e-state.json');

interface E2eState {
  pid: number;
  port: number;
  dbUrl: string;
}

/**
 * Read .env.e2e once and merge into the current process so spawned children
 * inherit deterministic secrets and ports without polluting the user's shell.
 */
function loadEnvE2e(): Record<string, string> {
  const path = resolve(ROOT, '.env.e2e');
  const text = readFileSync(path, 'utf8');
  const out: Record<string, string> = {};
  for (const line of text.split(/\r?\n/)) {
    const t = line.trim();
    if (!t || t.startsWith('#')) continue;
    const eq = t.indexOf('=');
    if (eq < 0) continue;
    const k = t.slice(0, eq).trim();
    let v = t.slice(eq + 1).trim();
    if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1);
    out[k] = v;
  }
  return out;
}

async function waitForHealth(url: string, timeoutMs = 15_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastErr: unknown = undefined;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(url);
      if (r.ok) return;
    } catch (e) {
      lastErr = e;
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`server did not become healthy at ${url}: ${String(lastErr)}`);
}

export default async function globalSetup(): Promise<void> {
  const env = loadEnvE2e();
  const port = Number(env.APP_PORT ?? 18080);
  const dbUrl = env.DATABASE_URL ?? 'database/database-e2e.sqlite';
  const dbPath = resolve(ROOT, dbUrl);

  // Fresh DB per run.
  for (const suffix of ['', '-shm', '-wal']) {
    const p = `${dbPath}${suffix}`;
    if (existsSync(p)) rmSync(p);
  }
  mkdirSync(resolve(ROOT, 'database'), { recursive: true });

  const childEnv = { ...process.env, ...env };

  // Build the binary once. Skip when already built (developers iterating).
  if (!process.env.E2E_SKIP_BUILD) {
    process.stdout.write('[e2e] cargo build...\n');
    execSync('cargo build --bin nineties', { cwd: ROOT, stdio: 'inherit', env: childEnv });
  }

  // Build frontend assets so /signin renders fully.
  if (!process.env.E2E_SKIP_FRONTEND) {
    process.stdout.write('[e2e] vite build...\n');
    execSync('npm run build', { cwd: ROOT, stdio: 'inherit', env: childEnv });
  }

  // Migrations + seed (legacy users for /signin form flow).
  process.stdout.write('[e2e] migrate + seed...\n');
  execSync('./target/debug/nineties migrate', { cwd: ROOT, stdio: 'inherit', env: childEnv });
  execSync('./target/debug/nineties seed', { cwd: ROOT, stdio: 'inherit', env: childEnv });

  // Spawn the server.
  process.stdout.write(`[e2e] spawning server on :${port}...\n`);
  const proc: ChildProcess = spawn('./target/debug/nineties', ['serve'], {
    cwd: ROOT,
    env: childEnv,
    stdio: ['ignore', 'inherit', 'inherit'],
    detached: false,
  });

  if (!proc.pid) {
    throw new Error('failed to spawn nineties server');
  }

  // Persist state for teardown.
  const state: E2eState = { pid: proc.pid, port, dbUrl };
  writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));

  // Wait until /health AND /__diag__/health both respond.
  await waitForHealth(`http://127.0.0.1:${port}/health`);
  await waitForHealth(`http://127.0.0.1:${port}/__diag__/health`);
  process.stdout.write('[e2e] server ready\n');
}
