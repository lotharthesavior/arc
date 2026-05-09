import { test as base, expect, APIRequestContext } from '@playwright/test';
import { randomUUID } from 'node:crypto';

export interface AuditFields {
  actor_id: string;
  actor_session_id: string | null;
  source_ip: string | null;
  user_agent: string | null;
  timestamp_utc_us: number;
  causation_id: string | null;
  correlation_id: string;
}

export interface DiagEvent {
  event_id: string;
  aggregate_type: string;
  aggregate_id: string;
  sequence: number;
  event_type: string;
  payload: Record<string, unknown>;
  audit: AuditFields;
}

export interface AuditApi {
  /** Return every event for the aggregate (typed) via the diag endpoint. */
  events(aggregateId: string): Promise<DiagEvent[]>;
}

interface Fixtures {
  /** Per-test unique email so registrations don't collide. */
  uniqueEmail: () => string;
  /** Per-test unique correlation id (caller can echo via X-Correlation-Id). */
  uniqueCorrelationId: () => string;
  /** Wraps the diag endpoint in a typed helper. */
  audit: AuditApi;
}

export const test = base.extend<Fixtures>({
  uniqueEmail: async ({}, use) => {
    await use(() => `e2e-${randomUUID().slice(0, 8)}@example.com`);
  },

  uniqueCorrelationId: async ({}, use) => {
    await use(() => randomUUID());
  },

  audit: async ({ request }, use) => {
    const api: AuditApi = {
      async events(aggregateId: string): Promise<DiagEvent[]> {
        const r = await request.get(`/__diag__/events/${aggregateId}`);
        if (!r.ok()) {
          throw new Error(`diag/events ${aggregateId} failed: ${r.status()} ${await r.text()}`);
        }
        const body = (await r.json()) as { events: DiagEvent[] };
        return body.events;
      },
    };
    await use(api);
  },
});

export { expect };

/** Convenience: extract the JSON `id` field from a 201 register response. */
export async function aggregateIdFromRegister(
  request: APIRequestContext,
  email: string,
  password = 'pw12345678',
  name = 'E2E User',
  headers: Record<string, string> = {},
): Promise<string> {
  const r = await request.post('/api/v1/register', {
    data: { name, email, password },
    headers,
  });
  expect(r.status(), `register: ${await r.text()}`).toBe(201);
  const body = (await r.json()) as { id: string };
  return body.id;
}
