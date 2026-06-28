import { test, expect } from '@playwright/test';
import { FRONTEND, MAILHOG, freshAccount, apiSignup } from './helpers';

/**
 * P0 go-live blocker (coverage gap): the email-verification click → auto-login
 * path. `/verify/[token]/+page.server.ts` POSTs /api/auth/verify-email, then
 * hand-parses the backend's Set-Cookie and forwards it via cookies.set before
 * redirecting to `/`. Every other spec SQL-shortcuts verification, so this real
 * link → verify → set-cookie-forward → auto-login chain was previously unproven.
 *
 * Requires MailHog (the dev SMTP sink) reachable at MAILHOG.
 */

/** Poll MailHog for the most recent message to `recipient` and return the
 *  first link matching `prefix` (e.g. "/verify"). Retries to absorb SMTP lag. */
async function latestMailLink(
  request: import('@playwright/test').APIRequestContext,
  recipient: string,
  prefix: string
): Promise<string | null> {
  for (let attempt = 0; attempt < 20; attempt++) {
    const r = await request.get(`${MAILHOG}/api/v2/messages`);
    if (r.ok()) {
      const json = (await r.json()) as {
        items: { Content: { Headers: { To?: string[] }; Body: string } }[];
      };
      const msg = json.items.find((m) => m.Content.Headers.To?.[0]?.includes(recipient));
      if (msg) {
        const re = new RegExp(`${prefix}/[A-Za-z0-9_\\-]+`);
        // The body may be plain, quoted-printable (soft-wrapped with "=\r\n"),
        // or base64 (our verify mail is base64). Try each decoding; return the
        // first matching PATH (host is ignored — we prepend FRONTEND).
        const raw = msg.Content.Body;
        const candidates = [raw, raw.replace(/=\r?\n/g, '')];
        try {
          candidates.push(Buffer.from(raw.replace(/\s/g, ''), 'base64').toString('utf8'));
        } catch {
          /* not base64 */
        }
        for (const text of candidates) {
          const m = text.match(re);
          if (m) return m[0];
        }
      }
    }
    await new Promise((res) => setTimeout(res, 500));
  }
  return null;
}

test('[verify-email] real /verify/<token> link auto-logs the user in (set-cookie forwarded)', async ({
  page,
  request
}) => {
  const acc = freshAccount(Date.now(), 'verify');
  await apiSignup(request, acc); // sends a verification email; user starts UNVERIFIED

  // Pull the real verification link from MailHog — NOT a SQL shortcut.
  const link = await latestMailLink(request, acc.email, '/verify');
  expect(link, 'verification email with a /verify/<token> link should arrive').toBeTruthy();

  // Navigate the link exactly as a user clicking it from their inbox would.
  await page.goto(`${FRONTEND}${link!}`);

  // The load forwards the backend session cookie and redirect(303, '/').
  await page.waitForURL(`${FRONTEND}/`);

  // Auto-login proven two ways: (1) a session cookie now exists on the
  // frontend origin (the hand-rolled Set-Cookie forward worked)...
  const cookies = await page.context().cookies();
  expect(
    cookies.some((c) => c.name === 'session' || c.name === '__Host-session'),
    'a session cookie should be set after verification'
  ).toBe(true);

  // ...and (2) the authenticated header affordance renders: the owner-only
  // Upload link is present and the anon Sign-in link is gone. (Two Upload
  // links exist — desktop + mobile nav — so scope to the header and take first.)
  const header = page.locator('header.app-header');
  await expect(header.locator('a[href="/upload"]').first()).toBeVisible();
  await expect(header.locator('a[href="/signin"]')).toHaveCount(0);
});
