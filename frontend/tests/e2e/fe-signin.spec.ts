import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup } from './helpers';

test.describe('signin edge cases', () => {
  test('[FE-0100][FE-0111] empty email submit reaches server guard and renders form-error', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/signin`);

    // FE-0111: inputs are typed and required (HTML validation).
    await expect(page.locator('input[name="email"]')).toHaveAttribute('type', 'email');
    await expect(page.locator('input[name="email"]')).toHaveAttribute('required', '');
    await expect(page.locator('input[name="password"]')).toHaveAttribute('type', 'password');

    // FE-0100: the server guard `if (!email || !password)` is unreachable while the
    // browser enforces `required`; strip it so the empty-email POST hits the action.
    await page.locator('input[name="email"]').evaluate((el) => el.removeAttribute('required'));
    await page.fill('input[name="password"]', 'whatever123');
    await page.click('button[type="submit"]');

    // FE-0100 + FE-0111: fail(400) message rendered as <p class="t-meta form-error">.
    // The submit is a non-enhanced full-page POST; let the locator auto-wait
    // through the navigation + SSR re-render (generous timeout absorbs the
    // round-trip under full-suite load — networkidle is racy/discouraged here).
    const err = page.locator('p.t-meta.form-error');
    await expect(err).toHaveText('Email and password are required.', { timeout: 15000 });
  });

  test('[FE-0104][FE-0113] unverified account with correct creds redirects to check-email and leaks the address', async ({
    page,
    request
  }) => {
    const acc = freshAccount(Date.now(), 'signin');
    await apiSignup(request, acc); // created, but email_verified_at stays NULL

    await page.goto(`${FRONTEND}/signin`);
    await page.fill('input[name="email"]', acc.email);
    await page.fill('input[name="password"]', acc.password);
    await page.click('button[type="submit"]');

    // FE-0104: the 403 path throws redirect(303) to /signup/check-email.
    await page.waitForURL(/\/signup\/check-email/);
    // FE-0113: the address is echoed in the query string (enumeration leak).
    const url = new URL(page.url());
    expect(url.pathname).toBe('/signup/check-email');
    expect(url.searchParams.get('email')).toBe(acc.email);
  });
});
