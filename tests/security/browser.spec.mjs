import { test, expect } from '@playwright/test';
const baseURL = process.env.ARC_SECURITY_URL || 'http://127.0.0.1:18764';
const desired = process.env.ARC_SECURITY_CONTRACTS === '1';
async function reset(request) {
  for (const name of ['alice','bob']) expect((await request.post(`/fixture/change/${name}/reset`)).status()).toBe(204);
}
test.beforeEach(async ({request}) => { await reset(request); });
for (const change of ['remove-role', 'disable']) {
  test(`GAP stale browser identity after ${change}; fresh JWT roles reject`, async ({ page, request }) => {
    const login = await (await page.goto('/fixture/login/alice')).json();
    expect((await page.goto('/browser/admin')).status()).toBe(200);
    expect((await request.post(`/fixture/change/alice/${change}`)).status()).toBe(204);
    const jwt = await request.get('/api/admin', {headers:{Authorization:`Bearer ${login.token}`}});
    expect(jwt.status()).toBe(403);
    const browserResponse = await page.goto('/browser/admin');
    expect(browserResponse.status(), 'GAP: cached role/active state is still accepted; no invalidation policy implemented').toBe(desired ? 403 : 200);
  });
}
test('idle expiration purges browser identity; GAP saved logout cookie replay', async ({page, context}) => {
  await page.goto('/fixture/login/alice');
  const saved = await context.cookies();
  expect(saved.some(c => c.httpOnly)).toBe(true);
  await page.request.post('/fixture/logout');
  await page.goto('/browser/admin');
  await expect(page).toHaveURL(/\/signin$/);
  await context.addCookies(saved);
  expect((await page.goto('/browser/admin')).status(), 'GAP: previously issued cookie remains replayable').toBe(desired ? 403 : 200);
  await page.request.post('/fixture/expire');
  await page.goto('/browser/admin');
  await expect(page).toHaveURL(/\/signin\?reason=idle$/);
});
async function openSocket(page) {
  await page.evaluate(() => {
    window.messages = [];
    window.socket = new WebSocket(`ws://${location.host}/ws`);
    socket.onmessage = e => messages.push(e.data);
    window.opened = new Promise(resolve => { socket.onopen = () => resolve(true); socket.onerror = () => resolve(false); });
  });
  return page.evaluate(() => window.opened);
}
async function subscribe(page, room) {
  await page.evaluate(room => { socket.send(JSON.stringify({type:'subscribe',room})); socket.send(JSON.stringify({type:'ping'})); },room);
  await expect.poll(() => page.evaluate(() => messages.includes('{"type":"pong"}'))).toBe(true);
}
test('GAP anonymous socket can subscribe to private room', async ({page,request}) => {
  await page.goto('/');
  const opened = await openSocket(page);
  if (desired) { expect(opened, 'desired authenticated handshake').toBe(false); return; }
  expect(opened).toBe(true);
  await subscribe(page, 'private');
  await request.post('/fixture/room/private');
  await expect.poll(() => page.evaluate(() => messages)).toContain('private-room-message');
  await page.evaluate(() => socket.close());
});
test('user broadcasts isolate users; GAP arbitrary room permits cross-user disclosure', async ({browser, request}) => {
  const alice = await browser.newContext(), bob = await browser.newContext();
  try {
    const a = await alice.newPage(), b = await bob.newPage();
    const user = await (await a.goto(`${baseURL}/fixture/login/alice`)).json();
    await b.goto(`${baseURL}/fixture/login/bob`);
    expect(await openSocket(a)).toBe(true); expect(await openSocket(b)).toBe(true);
    await subscribe(a, `user:${user.id}`); await subscribe(b, `user:${user.id}`);
    await request.post(`/fixture/user/${user.id}`);
    await expect.poll(() => a.evaluate(() => messages)).toContain('private-user-message');
    // Ordered round-trip barrier then check Bob, instead of a negative sleep assertion.
    const pongCount = await b.evaluate(() => messages.filter(m => m === '{"type":"pong"}').length);
    await b.evaluate(() => socket.send(JSON.stringify({type:'ping'})));
    await expect.poll(() => b.evaluate(() => messages.filter(m => m === '{"type":"pong"}').length)).toBe(pongCount + 1);
    expect(await b.evaluate(() => messages)).not.toContain('private-user-message');
    await request.post(`/fixture/room/${encodeURIComponent(`user:${user.id}`)}`);
    await expect.poll(() => b.evaluate(() => messages)).toContain('private-room-message');
    if (desired) expect(await b.evaluate(() => messages), 'desired room authorization').not.toContain('private-room-message');
  } finally { await alice.close(); await bob.close(); }
});
