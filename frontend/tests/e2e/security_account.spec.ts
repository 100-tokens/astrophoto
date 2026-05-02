/**
 * E2E tests for security / account management flows.
 *
 * NOTE: These tests require the full dev stack running (`just dev`) and a
 * MailHog instance reachable at http://localhost:8025.
 *
 * A `playwright.config.ts` at `frontend/` root is needed to execute these.
 * That config is deferred — add it when wiring up CI Playwright runs.
 * Until then this file is a spec stub: types-check via pnpm check but
 * won't be picked up by any test runner.
 */

import { test, expect } from '@playwright/test';

const MAILHOG = 'http://localhost:8025';
const BACKEND = 'http://localhost:8080';
const FRONTEND = 'http://localhost:5173';

/** Pull the most recent email link for a given recipient and URL prefix. */
async function latestMailLink(
  page: import('@playwright/test').Page,
  recipient: string,
  prefix: string
): Promise<string | null> {
  const r = await page.request.get(`${MAILHOG}/api/v2/messages`);
  const json = await r.json();
  const msgs = (json.items as unknown[]).filter((m: unknown) =>
    (m as { Content: { Headers: { To?: string[] } } }).Content.Headers.To?.[0]?.includes(recipient)
  );
  if (!msgs.length) return null;
  const body = (msgs[0] as { Content: { Body: string } }).Content.Body;
  const match = body.match(new RegExp(`${prefix}/[A-Za-z0-9_-]+`));
  return match ? match[0] : null;
}

test('reset password from sign-in, click MailHog link, set new password, land authenticated', async ({
  page,
  request
}) => {
  const email = `e2e-${Date.now()}@reset.test`;

  // Create account via API.
  await request.post(`${BACKEND}/api/auth/signup`, {
    data: { email, password: 'longenoughpw1', display_name: 'E2E' }
  });

  // Request password reset from the UI.
  await page.goto(`${FRONTEND}/reset`);
  await page.getByLabel('EMAIL').fill(email);
  await page.getByRole('button', { name: 'Send reset link' }).click();
  await expect(page).toHaveURL(/\/reset\/sent/);

  // Pull the reset link from MailHog.
  const link = await latestMailLink(page, email, '/reset');
  expect(link).toBeTruthy();
  await page.goto(`${FRONTEND}${link!}`);

  // Set the new password.
  await page.getByLabel('NEW PASSWORD').fill('a-strong-new-password-x9');
  await page.getByRole('button', { name: 'Set new password & sign in' }).click();
  await expect(page).toHaveURL(/\/$/);
});

test('toggle theme persists across reload', async ({ page }) => {
  // NOTE: this test requires the user to be signed in. Add a sign-in fixture
  // (shared across tests) when playwright.config.ts is wired up.
  await page.goto(`${FRONTEND}/settings/appearance`);
  // If redirected to sign-in, skip the assertion — fixture not yet in place.
  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }
  await page.getByRole('button', { name: 'LIGHT' }).click();
  await page.reload();
  const theme = await page.locator('html').getAttribute('data-theme');
  expect(theme).toBe('light');
});
