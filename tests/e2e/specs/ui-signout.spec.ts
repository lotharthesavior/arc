import { test, expect } from '../fixtures';

async function signinSeeded(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/signin');
  await page.fill('input[name="email"]', 'jekyll@example.com');
  await page.fill('input[name="password"]', 'password');
  await page.click('button[type="submit"]');
  await page.waitForURL(/\/admin/);
}

test.describe('UI signout flow', () => {
  test('signout clears the session cookie and bounces /admin to /signin', async ({
    page,
    context,
  }) => {
    await signinSeeded(page);

    // Use the context request client so the redirect target's UI lifecycle
    // cannot obscure the protocol assertion. It shares the browser cookie jar.
    const signout = await context.request.get('/signout');
    expect(signout.ok()).toBeTruthy();

    // Cookie either gone or replaced — definitely no longer authenticates.
    await page.goto('/admin');
    await expect(page).toHaveURL(/\/signin/);
  });
});
