import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { test } from '@playwright/test';

test('security headers preserve Vite assets, caching, Turbo and Toastify', async ({ baseURL }) => {
  execFileSync(process.execPath, [
    resolve(__dirname, '../../..', 'scripts/check-security-assets.mjs'),
    baseURL!,
  ], { stdio: 'inherit', timeout: 45_000 });
});
