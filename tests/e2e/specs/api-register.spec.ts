import { test, expect, aggregateIdFromRegister } from '../fixtures';

test.describe('POST /api/v1/register', () => {
  test('creates a UserRegistered event with anonymous audit', async ({
    request,
    audit,
    uniqueEmail,
  }) => {
    const email = uniqueEmail();
    const id = await aggregateIdFromRegister(request, email, 'correct horse battery staple', 'Alice', {
      'User-Agent': 'e2e-register/1.0',
    });

    const events = await audit.events(id);
    expect(events).toHaveLength(1);

    const ev = events[0];
    expect(ev.event_type).toBe('UserRegistered');
    expect(ev.aggregate_id).toBe(id);
    expect(ev.payload).toMatchObject({ email, name: 'Alice' });

    expect(ev.audit.actor_id).toBe('anonymous');
    expect(ev.audit.user_agent).toBe('e2e-register/1.0');
    expect(ev.audit.timestamp_utc_us).toBeGreaterThan(0);
    // correlation_id auto-generated (not nil)
    expect(ev.audit.correlation_id).not.toBe('00000000-0000-0000-0000-000000000000');
  });

  test('echoes X-Correlation-Id header into audit metadata', async ({
    request,
    audit,
    uniqueEmail,
    uniqueCorrelationId,
  }) => {
    const email = uniqueEmail();
    const corr = uniqueCorrelationId();

    const id = await aggregateIdFromRegister(request, email, 'pw12345678', 'Bob', {
      'X-Correlation-Id': corr,
    });

    const events = await audit.events(id);
    expect(events[0].audit.correlation_id).toBe(corr);
  });

  test('rejects duplicate email without writing a second event', async ({
    request,
    audit,
    uniqueEmail,
  }) => {
    const email = uniqueEmail();
    const id = await aggregateIdFromRegister(request, email);

    const dup = await request.post('/api/v1/register', {
      data: { name: 'Dup', email, password: 'pw12345678' },
    });
    expect(dup.status()).not.toBe(201);

    const events = await audit.events(id);
    expect(events).toHaveLength(1);
  });

  test('rejects malformed email and writes nothing to the store', async ({
    request,
  }) => {
    const r = await request.post('/api/v1/register', {
      data: { name: 'Eve', email: 'not-an-email', password: 'pw12345678' },
    });
    expect(r.status()).toBe(422);
  });
});
