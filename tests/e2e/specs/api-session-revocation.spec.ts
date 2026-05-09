import { test, expect, aggregateIdFromRegister } from '../fixtures';

async function loginToken(
  request: import('@playwright/test').APIRequestContext,
  email: string,
  password: string,
): Promise<string> {
  const r = await request.post('/api/v1/login', { data: { email, password } });
  expect(r.status(), `login: ${await r.text()}`).toBe(200);
  return ((await r.json()) as { token: string }).token;
}

test.describe('HIPAA-4 session revocation', () => {
  test('logout invalidates the bearer token', async ({ request, uniqueEmail }) => {
    const email = uniqueEmail();
    const password = 'pw12345678';

    await aggregateIdFromRegister(request, email, password, 'Janet');
    const token = await loginToken(request, email, password);
    const auth = { Authorization: `Bearer ${token}` };

    // Token works.
    const before = await request.get('/api/v1/protected/profile', { headers: auth });
    expect(before.status()).toBe(200);

    // Revoke via /logout.
    const out = await request.post('/api/v1/protected/logout', { headers: auth });
    expect(out.status()).toBe(204);

    // Same bearer no longer authorizes.
    const after = await request.get('/api/v1/protected/profile', { headers: auth });
    expect(after.status()).toBe(401);
  });

  test('two issued tokens are independently revocable', async ({ request, uniqueEmail }) => {
    const email = uniqueEmail();
    const password = 'pw12345678';

    await aggregateIdFromRegister(request, email, password, 'Karl');
    const tokenA = await loginToken(request, email, password);
    const tokenB = await loginToken(request, email, password);
    expect(tokenA).not.toBe(tokenB);

    // Revoke A only.
    const out = await request.post('/api/v1/protected/logout', {
      headers: { Authorization: `Bearer ${tokenA}` },
    });
    expect(out.status()).toBe(204);

    // A is dead, B still alive.
    const aAfter = await request.get('/api/v1/protected/profile', {
      headers: { Authorization: `Bearer ${tokenA}` },
    });
    expect(aAfter.status()).toBe(401);

    const bAfter = await request.get('/api/v1/protected/profile', {
      headers: { Authorization: `Bearer ${tokenB}` },
    });
    expect(bAfter.status()).toBe(200);
  });
});
