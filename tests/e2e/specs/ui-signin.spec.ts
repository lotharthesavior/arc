import { test, expect } from '../fixtures';

test.describe('UI signin flow', () => {
  test('seeded user can sign in via the form and reach /admin', async ({ page, context }) => {
    await page.goto('/signin');
    await expect(page).toHaveURL(/\/signin$/);

    // CSRF token is rendered as a hidden input.
    const csrfToken = await page.locator('input[name="csrf_token"]').inputValue();
    expect(csrfToken).not.toBe('');

    await page.fill('input[name="email"]', 'jekyll@example.com');
    await page.fill('input[name="password"]', 'password');
    await page.click('button[type="submit"]');

    await page.waitForURL(/\/admin/);
    await expect(page).toHaveURL(/\/admin/);

    const cookies = await context.cookies();
    expect(cookies.find((c) => c.name === 'arc_session')).toBeTruthy();
  });

  test('wrong password keeps user on /signin', async ({ page }) => {
    await page.goto('/signin');
    const csrfToken = await page.locator('input[name="csrf_token"]').inputValue();
    expect(csrfToken).not.toBe('');

    await page.fill('input[name="email"]', 'jekyll@example.com');
    await page.fill('input[name="password"]', 'wrong-password');
    await page.click('button[type="submit"]');

    // SeeOther → /signin
    await page.waitForURL(/\/signin/);
    await expect(page.locator('body')).toContainText(/invalid credentials/i);
  });

  test('CSRF token mismatch rejects submission', async ({ page, request }) => {
    await page.goto('/signin');
    // Submit raw form without going through the rendered page (no CSRF cookie alignment).
    const r = await request.post('/signin', {
      form: {
        csrf_token: 'tampered-value',
        email: 'jekyll@example.com',
        password: 'password',
      },
      maxRedirects: 0,
    });
    // Server should not transition to /admin; the redirect target stays /signin.
    expect(r.status()).toBe(303);
    expect(r.headers()['location']).toMatch(/\/signin/);
  });
});
