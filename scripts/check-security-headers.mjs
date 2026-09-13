import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';

const baseUrl = process.argv[2];
assert(baseUrl, 'usage: node scripts/check-security-headers.mjs <generated-app-url>');
const csp = "base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'";
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH || undefined,
});
try {
  const page = await browser.newPage();
  for (const [path, status] of [['/health', 200], ['/signin', 200], ['/admin', 302], ['/missing', 404], ['/public/styles.css', 200], ['/api/products', 401]]) {
    const response = await page.request.get(`${baseUrl}${path}`, { maxRedirects: 0 });
    assert.equal(response.status(), status, path);
    assert.equal(response.headers()['content-security-policy'], csp, path);
    assert.equal(response.headers()['x-content-type-options'], 'nosniff', path);
    assert.equal(response.headers()['x-frame-options'], 'DENY', path);
    assert.equal(response.headers()['referrer-policy'], 'strict-origin-when-cross-origin', path);
    assert.equal(response.headers()['permissions-policy'], 'camera=(), microphone=(), geolocation=()', path);
    assert.equal(response.headers()['strict-transport-security'], undefined, path);
  }
  const csrf = await page.request.post(`${baseUrl}/signin`, {
    form: { csrf_token: 'invalid', email: 'admin@example.com', password: 'wrong' },
  });
  assert.equal(csrf.status(), 403);
  assert.equal(csrf.headers()['content-security-policy'], csp);

  const login = await page.request.post(`${baseUrl}/api/session`, {
    data: { email: 'admin@example.com', password: 'change-me-now' },
  });
  assert.equal(login.status(), 200, 'JWT login');
  assert.equal(login.headers()['content-security-policy'], csp);
  const { token } = await login.json();
  assert(token, 'JWT token issued');
  const authorized = await page.request.get(`${baseUrl}/api/products`, {
    headers: { authorization: `Bearer ${token}` },
  });
  assert.equal(authorized.status(), 200, 'JWT-authorized resource request');
  assert.equal(authorized.headers()['content-security-policy'], csp);

  await page.goto(`${baseUrl}/signin`);
  await page.evaluate(() => {
    window.securityViolations = [];
    document.addEventListener('securitypolicyviolation', e => window.securityViolations.push(e.effectiveDirective));
    const base = document.createElement('base');
    base.href = 'https://example.invalid/';
    document.head.append(base);
  });
  await page.waitForFunction(() => window.securityViolations.includes('base-uri'));
  assert.equal(await page.evaluate(() => document.baseURI), `${baseUrl}/signin`);
  await page.evaluate(() => {
    const form = document.createElement('form');
    form.action = 'https://example.invalid/blocked';
    form.method = 'post';
    document.body.append(form);
    form.submit();
  });
  await page.waitForFunction(() => window.securityViolations.includes('form-action'));
  assert.equal(page.url(), `${baseUrl}/signin`);

  // Even a same-origin parent cannot frame the application.
  const frame = await browser.newPage();
  await frame.goto(`${baseUrl}/signin`);
  const blocked = frame.waitForEvent('console', {
    predicate: message => /frame-ancestors|X-Frame-Options/i.test(message.text()),
  });
  await frame.evaluate(() => {
    const iframe = document.createElement('iframe');
    iframe.src = '/signin';
    document.body.append(iframe);
  });
  await blocked;
  console.log('Security headers: redirects, auth, CSRF, API, assets, base/form/frame enforcement passed.');
} finally {
  await browser.close();
}
