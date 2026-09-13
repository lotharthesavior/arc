import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: '.', testMatch: 'browser.spec.mjs', workers: 1, retries: 0,
  timeout: 15000, outputDir: '../../test-results/security-coverage',
  reporter: 'list',
  use: { baseURL: process.env.ARC_SECURITY_URL || 'http://127.0.0.1:18764', headless: true,
    launchOptions: { executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', args: ['--no-sandbox'] } }
});
