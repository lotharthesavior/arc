import { existsSync, readFileSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

const ROOT = resolve(__dirname, '../..');
const STATE_FILE = resolve(ROOT, '.e2e-state.json');

interface E2eState {
  pid: number;
  port: number;
  dbUrl: string;
}

export default async function globalTeardown(): Promise<void> {
  if (!existsSync(STATE_FILE)) return;
  const state: E2eState = JSON.parse(readFileSync(STATE_FILE, 'utf8'));

  try {
    process.kill(state.pid, 'SIGTERM');
  } catch (e) {
    // Process may already have exited.
  }

  // Wait briefly for graceful shutdown.
  for (let i = 0; i < 25; i++) {
    try {
      process.kill(state.pid, 0);
      await new Promise((r) => setTimeout(r, 100));
    } catch {
      break;
    }
  }
  // Hard kill if still alive.
  try {
    process.kill(state.pid, 'SIGKILL');
  } catch {}

  // Clean up the DB file.
  const dbPath = resolve(ROOT, state.dbUrl);
  for (const suffix of ['', '-shm', '-wal']) {
    const p = `${dbPath}${suffix}`;
    if (existsSync(p)) rmSync(p);
  }

  rmSync(STATE_FILE, { force: true });
}
