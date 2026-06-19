/**
 * E2E tests for security / account management flows.
 *
 * Requires the full dev stack running and a MailHog instance reachable at
 * http://localhost:8025 (PUBLIC_BASE_URL points reset links at :5180).
 */

import { test, expect } from '@playwright/test';
import { FRONTEND, MAILHOG, freshAccount, apiSignup, signupVerifiedAndLogin } from './helpers';

/**
 * Pull the most recent email link for a given recipient and URL path prefix.
 * The mailer encodes bodies as base64 (Content-Transfer-Encoding: base64), so
 * decode before matching. Returns the URL path ("/reset/<token>") with any
 * scheme/host stripped, so it can be reattached to the frontend origin.
 */
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
  const raw = (msgs[0] as { Content: { Body: string } }).Content.Body;
  let body = raw;
  try {
    body = Buffer.from(raw.replace(/\r?\n/g, ''), 'base64').toString('utf8');
  } catch {
    // Not base64 — fall through and match the raw body.
  }
  const match = body.match(new RegExp(`${prefix}/[A-Za-z0-9_-]+`));
  return match ? match[0] : null;
}

test('reset password from sign-in, click MailHog link, set new password, land authenticated', async ({
  page,
  request
}) => {
  const acc = freshAccount(Date.now(), 'reset');

  // Create account via API (signup requires a unique handle).
  await apiSignup(request, acc);

  // Request password reset from the UI.
  await page.goto(`${FRONTEND}/reset`);
  await page.getByLabel('EMAIL').fill(acc.email);
  await page.getByRole('button', { name: 'Send reset link' }).click();
  await expect(page).toHaveURL(/\/reset\/sent/);

  // Pull the reset link from MailHog. The mail send is async; poll briefly.
  let link: string | null = null;
  await expect
    .poll(
      async () => {
        link = await latestMailLink(page, acc.email, '/reset');
        return link;
      },
      { timeout: 10000, intervals: [250] }
    )
    .toBeTruthy();
  await page.goto(`${FRONTEND}${link!}`);

  // Set the new password.
  await page.getByLabel('NEW PASSWORD').fill('a-strong-new-password-x9');
  await page.getByRole('button', { name: 'Set new password & sign in' }).click();
  await expect(page).toHaveURL(/\/$/);
});

test('toggle theme persists across reload', async ({ page, request }) => {
  // /settings/* is auth-gated (settings/+layout.server.ts redirects to /signin),
  // so sign in first. The theme is stored in a cookie and read by app.html at
  // SSR (data-theme); clicking LIGHT POSTs setTheme then redirects back.
  await signupVerifiedAndLogin(page, request, Date.now(), 'theme');
  await page.goto(`${FRONTEND}/settings/appearance`);
  await page.getByRole('button', { name: 'LIGHT' }).click();
  await page.reload();
  const theme = await page.locator('html').getAttribute('data-theme');
  expect(theme).toBe('light');
});
