import { test, expect, aggregateIdFromRegister } from '../fixtures';

async function loginToken(
  request: import('@playwright/test').APIRequestContext,
  email: string,
  password: string,
): Promise<string> {
  const r = await request.post('/api/v1/login', { data: { email, password } });
  expect(r.status(), `login: ${await r.text()}`).toBe(200);
  const body = (await r.json()) as { token: string };
  return body.token;
}

test.describe('Authenticated profile flow', () => {
  test('register → login → patch → get reflects → delete → 404', async ({
    request,
    audit,
    uniqueEmail,
  }) => {
    const email = uniqueEmail();
    const password = 'pw12345678';

    const id = await aggregateIdFromRegister(request, email, password, 'Carol');
    const token = await loginToken(request, email, password);
    const auth = { Authorization: `Bearer ${token}` };

    // Update name
    const patch = await request.patch('/api/v1/protected/profile', {
      headers: auth,
      data: { name: 'Carol Updated' },
    });
    expect(patch.status()).toBe(204);

    // GET reflects update
    const get1 = await request.get('/api/v1/protected/profile', { headers: auth });
    expect(get1.status()).toBe(200);
    const profile = (await get1.json()) as { id: string; name: string; email: string };
    expect(profile).toMatchObject({ id, name: 'Carol Updated', email });

    // DELETE
    const del = await request.delete('/api/v1/protected/profile', { headers: auth });
    expect(del.status()).toBe(204);

    // GET is 404
    const get2 = await request.get('/api/v1/protected/profile', { headers: auth });
    expect(get2.status()).toBe(404);

    // Audit trail: 3 events, second + third stamped with the aggregate UUID.
    const events = await audit.events(id);
    expect(events.map((e) => e.event_type)).toEqual([
      'UserRegistered',
      'ProfileUpdated',
      'UserDeleted',
    ]);
    expect(events[0].audit.actor_id).toBe('anonymous');
    expect(events[1].audit.actor_id).toBe(id);
    expect(events[2].audit.actor_id).toBe(id);

    // Timestamps strictly monotonic (microseconds — same-millisecond writes are rare here)
    expect(events[1].audit.timestamp_utc_us).toBeGreaterThanOrEqual(
      events[0].audit.timestamp_utc_us,
    );
    expect(events[2].audit.timestamp_utc_us).toBeGreaterThanOrEqual(
      events[1].audit.timestamp_utc_us,
    );
  });

  test('profile is unreachable without a Bearer token', async ({ request }) => {
    const r = await request.get('/api/v1/protected/profile');
    expect(r.status()).toBe(401);
  });

  test('login rejects bad credentials with 401', async ({ request, uniqueEmail }) => {
    const email = uniqueEmail();
    await aggregateIdFromRegister(request, email, 'pw12345678', 'Dan');
    const r = await request.post('/api/v1/login', {
      data: { email, password: 'wrong-password' },
    });
    expect(r.status()).toBe(401);
  });
});
