import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import { FRONTEND, freshAccount, apiSignup, signupVerifiedAndLogin, sql } from './helpers';

/**
 * Advance the installed Playwright clock past the 60s resend countdown until
 * the active "Resend link" button appears.
 *
 * Notes learned the hard way:
 * - `clock.fastForward` fires each timer at most once; `clock.runFor` fires
 *   the setInterval repeatedly, so runFor is required.
 * - A single runFor still races Svelte's interval registration / render flush,
 *   so retry until the countdown actually reaches 0.
 * - A registered `page.route` interception breaks the fake clock entirely;
 *   callers must NOT intercept this request (use passive listeners / offline).
 */
async function tickPastCountdown(
  page: import('@playwright/test').Page,
  expectedName: string | RegExp = 'Resend link'
): Promise<void> {
  const target = page.getByRole('button', { name: expectedName });
  for (let i = 0; i < 10; i++) {
    await page.clock.runFor(60000);
    if (await target.count()) return;
  }
  await expect(target).toBeVisible();
}

// ─────────────────────────────────────────────────────────────────────────
// /signup
// ─────────────────────────────────────────────────────────────────────────

test.describe('signup edge cases', () => {
  test('[FE-0114] empty required field → server guard fail(400) All fields are required', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/signup`);

    const acc = freshAccount(Date.now(), 'au2');
    // Fill everything except display_name, leaving it empty so the
    // `if (!email || !password || !display_name || !handle)` server guard fires.
    // Native form.submit() bypasses HTML `required` validation in one tick
    // (a strip+click races the Svelte <Input> re-render restoring `required`).
    await page.fill('input[name="handle"]', acc.handle);
    await page.fill('input[name="email"]', acc.email);
    await page.fill('input[name="password"]', acc.password);
    await Promise.all([
      page.waitForResponse((r) => r.url().endsWith('/signup') && r.request().method() === 'POST'),
      page.locator('form.signup-form').evaluate((el) => (el as HTMLFormElement).submit())
    ]);

    const err = page.locator('form.signup-form p.t-meta.form-error');
    await expect(err).toBeVisible();
    await expect(err).toHaveText('All fields are required.');
  });

  test('[FE-0115] 9-char password → front guard fail(400) min 10 chars before any network call', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/signup`);

    const acc = freshAccount(Date.now(), 'au2');
    await page.fill('input[name="display_name"]', acc.displayName);
    await page.fill('input[name="handle"]', acc.handle);
    await page.fill('input[name="email"]', acc.email);
    await page.fill('input[name="password"]', '123456789'); // exactly 9 chars
    await page.click('button[type="submit"]');
    await page.waitForLoadState('networkidle');

    const err = page.locator('form.signup-form p.t-meta.form-error');
    await expect(err).toBeVisible();
    await expect(err).toHaveText('Password must be at least 10 characters.');
  });

  test('[FE-0125][FE-0126] 409 handle-conflict → handleError under picker, handle value preserved', async ({
    page,
    request
  }) => {
    // Pre-create an account so its handle is already taken; the backend 409s
    // with message "conflict: handle already taken", which the action maps by
    // testing msg.includes('handle') → handleError (not the email message).
    const existing = freshAccount(Date.now(), 'au2');
    await apiSignup(request, existing);

    await page.goto(`${FRONTEND}/signup`);
    const dupe = freshAccount(Date.now(), 'au2'); // fresh email, but reuse handle
    await page.fill('input[name="display_name"]', dupe.displayName);
    await page.fill('input[name="handle"]', existing.handle); // collide
    await page.fill('input[name="email"]', dupe.email);
    await page.fill('input[name="password"]', dupe.password);
    await page.click('button[type="submit"]');
    await page.waitForLoadState('networkidle');

    // FE-0125: disambiguation by message text routes to the handleError branch.
    // FE-0126: handleError renders under the picker as <p class="t-meta form-error">.
    const handleErr = page.locator('.field p.t-meta.form-error');
    await expect(handleErr).toBeVisible();
    await expect(handleErr).toHaveText('That handle is already taken.');

    // FE-0126: the $effect rehydrates handle = form.handle (HandlePicker bind:value).
    await expect(page.locator('input[name="handle"]')).toHaveValue(existing.handle);

    // FE-0125: it is NOT the email-conflict message.
    await expect(page.locator('form.signup-form > p.t-meta.form-error')).toHaveCount(0);
  });

  test('[FE-0121] authenticated user visiting /signup → redirect(303) to /', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'au2');

    await page.goto(`${FRONTEND}/signup`);
    await page.waitForLoadState('networkidle');

    // load() throws redirect(303, '/') for a logged-in user: no signup form.
    expect(new URL(page.url()).pathname).toBe('/');
    await expect(page.locator('form.signup-form')).toHaveCount(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────
// /signup/check-email
// ─────────────────────────────────────────────────────────────────────────

test.describe('signup/check-email edge cases', () => {
  test('[FE-0129] resend action with empty hidden email → fail(400, missing_email) without calling backend', async ({
    request
  }) => {
    // The page never surfaces this fail in the DOM and the button is disabled
    // during the 60s countdown, so assert the action endpoint directly.
    // SvelteKit CSRF requires a matching origin header for form POSTs; the
    // `x-sveltekit-action` header makes the action return its structured
    // result (the fail status lives in the JSON, the HTTP status stays 200).
    const res = await request.post(`${FRONTEND}/signup/check-email?/resend`, {
      headers: { origin: FRONTEND, 'x-sveltekit-action': 'true' },
      form: { email: '' }
    });
    const result = JSON.parse(await res.text()) as { type: string; status: number; data: string };
    expect(result.type).toBe('failure');
    expect(result.status).toBe(400);
    expect(result.data).toContain('missing_email');
  });

  test('[FE-0130] resend Button disabled + "Resend in {n}s" during the first 60 seconds', async ({
    page
  }) => {
    const email = 'au2-0130@example.com';
    await page.clock.install();
    await page.goto(`${FRONTEND}/signup/check-email?email=${encodeURIComponent(email)}`);

    const btn = page.getByRole('button', { name: /Resend in \d+s/ });
    await expect(btn).toBeVisible();
    await expect(btn).toBeDisabled();
    await expect(btn).toHaveText(/Resend in 60s/);

    // The $effect setInterval decrements secondsLeft once per second; runFor
    // fires it repeatedly (3 ticks → 57s).
    await page.clock.runFor(3000);
    await expect(btn).toHaveText(/Resend in 57s/);
    await expect(btn).toBeDisabled();
  });

  test('[FE-0136] reflected ?email XSS neutralised by Svelte auto-escape; ?expired=1 shows the expiry warning', async ({
    page
  }) => {
    const payload = '<a href=x>pwn</a>';
    await page.goto(
      `${FRONTEND}/signup/check-email?email=${encodeURIComponent(payload)}&expired=1`
    );

    // email rendered via {email} inside <strong>{email}</strong>: literal, escaped.
    const strong = page.locator('p.t-body strong').first();
    await expect(strong).toHaveText(payload);
    // No injected anchor materialised from the payload.
    await expect(strong.locator('a')).toHaveCount(0);

    // {#if expired} branch renders the expired-link warning.
    await expect(
      page.getByText(/That verification link has expired or was already used/)
    ).toBeVisible();
  });
});

// ─────────────────────────────────────────────────────────────────────────
// /reset
// ─────────────────────────────────────────────────────────────────────────

test.describe('reset request edge cases', () => {
  test('[FE-0138] whitespace-only email → server trim guard fail(400) → "Please enter a valid email."', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset`);

    // type=email rejects whitespace and `required` blocks empties client-side.
    // Set the value and call the native form.submit() in one tick: .submit()
    // bypasses HTML constraint validation entirely (unlike requestSubmit()),
    // and doing it atomically avoids the Svelte <Input> re-render that would
    // otherwise restore type/required between a strip and a click. The server
    // then trims '   ' → empty → fail(400, missing_email).
    await Promise.all([
      page.waitForResponse((r) => r.url().endsWith('/reset') && r.request().method() === 'POST'),
      page.locator('input[name="email"]').evaluate((el) => {
        const i = el as HTMLInputElement;
        i.value = '   ';
        i.form?.submit();
      })
    ]);

    const err = page.locator('p.t-meta.form-error');
    await expect(err).toBeVisible();
    await expect(err).toHaveText('Please enter a valid email.');
  });

  test('[FE-0144] missing_email render: form-error <p>, required type=email field, back link to /signin', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset`);

    // Field contract holds on first render.
    const input = page.locator('input[name="email"]');
    await expect(input).toHaveAttribute('type', 'email');
    await expect(input).toHaveAttribute('required', '');
    await expect(page.locator('.back-link a[href="/signin"]')).toBeVisible();

    // Drive the missing_email fail to assert the {#if form?.error === 'missing_email'}
    // block. Native form.submit() bypasses HTML validation and serializes the
    // raw value in one tick (see FE-0138 for the rationale).
    await Promise.all([
      page.waitForResponse((r) => r.url().endsWith('/reset') && r.request().method() === 'POST'),
      input.evaluate((el) => {
        const i = el as HTMLInputElement;
        i.value = '   ';
        i.form?.submit();
      })
    ]);

    await expect(page.locator('p.t-meta.form-error')).toHaveText('Please enter a valid email.');
    // Back link still present after the round-trip.
    await expect(page.locator('.back-link a[href="/signin"]')).toBeVisible();
  });
});

// ─────────────────────────────────────────────────────────────────────────
// /reset/[token]
// ─────────────────────────────────────────────────────────────────────────

test.describe('reset/[token] edge cases', () => {
  test('[FE-0146] 11-char new_password → front guard fail(400, too_short) → "Password must be at least 12 characters."', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset/sometoken123`);

    // 11 chars: under the front guard `new_password.length < 12`.
    await page
      .locator('input[name="new_password"]')
      .evaluate((el) => el.removeAttribute('required'));
    await page.fill('input[name="new_password"]', '12345678901'); // 11 chars
    await page.click('button[type="submit"]');
    await page.waitForLoadState('networkidle');

    const err = page.locator('p.t-meta.form-error');
    await expect(err).toBeVisible();
    await expect(err).toHaveText('Password must be at least 12 characters.');
  });

  test('[FE-0153] strength bar segments light up as you type; "Use at least 12 characters" warning while < 12', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset/sometoken123`);
    const input = page.locator('input[name="new_password"]');
    const segs = page.locator('.strength-bar .strength-seg');
    const onSegs = page.locator('.strength-bar .strength-seg.on');
    const warn = page.locator('p.t-meta.warn');

    // strength(): 1 (<8), 2 (<12), 3 (<16), 4 (else). Bar shows once pwd.length > 0.
    await input.fill('abc'); // len 3 → strength 1
    await expect(segs).toHaveCount(4);
    await expect(onSegs).toHaveCount(1);
    await expect(warn).toHaveText('Use at least 12 characters.');

    await input.fill('abcdefghij'); // len 10 → strength 2
    await expect(onSegs).toHaveCount(2);
    await expect(warn).toBeVisible(); // still < 12

    await input.fill('abcdefghijklmn'); // len 14 → strength 3
    await expect(onSegs).toHaveCount(3);
    await expect(warn).toHaveCount(0); // >= 12, warning gone

    await input.fill('abcdefghijklmnop'); // len 16 → strength 4
    await expect(onSegs).toHaveCount(4);
  });
});

// ─────────────────────────────────────────────────────────────────────────
// /reset/sent
// ─────────────────────────────────────────────────────────────────────────

test.describe('reset/sent edge cases', () => {
  test('[FE-0155] direct nav without ?email → fallback "your inbox" headline, empty To: preview, no crash', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset/sent`);

    // email = '' → headline uses `email || 'your inbox'`.
    await expect(page.locator('h1.reset-headline')).toContainText('to your inbox');
    // The email preview renders with an empty To: and does not throw.
    const preview = page.locator('pre.email-preview');
    await expect(preview).toBeVisible();
    await expect(preview).toContainText('To:');
  });

  test('[FE-0156][FE-0157] countdown 60→0 (Button "Resend in 0:SS" disabled, then active); rapid clicks bounded to one request', async ({
    page
  }) => {
    // Count requests passively: page.route interception breaks the fake clock,
    // so we observe the wire instead. Offline (set after the countdown) makes
    // the resend fetch reject fast — the contract is about bounding the
    // double-submit, not the response — while still emitting one request event.
    let resetRequests = 0;
    page.on('request', (r) => {
      if (r.url().includes('/api/auth/password-reset/request')) resetRequests += 1;
    });

    await page.clock.install();
    await page.goto(`${FRONTEND}/reset/sent?email=someone@example.com`);

    // FE-0156: while secondsLeft > 0 the ghost Button is disabled, "Resend in 0:SS".
    // This also gates the clock: wait for the hydrated countdown before runFor,
    // else the interval may not be registered when the clock advances.
    const disabledBtn = page.getByRole('button', { name: /Resend in 0:\d{2}/ });
    await expect(disabledBtn).toBeVisible();
    await expect(disabledBtn).toBeDisabled();
    await expect(disabledBtn).toHaveText(/Resend in 0:60/); // padStart("60")

    // FE-0156: run past 60s → countdown hits 0, active "Resend link" button.
    await tickPastCountdown(page);
    const activeBtn = page.getByRole('button', { name: 'Resend link' });
    await expect(activeBtn).toBeVisible();
    await expect(activeBtn).toBeEnabled();

    // FE-0157: rapid clicks. resend() sets resending=true on the first click,
    // and secondsLeft=60 — so the Button is immediately disabled and renamed,
    // which is what BOUNDS the double-submission (the browser drops clicks on a
    // disabled button). Offline makes the resend fetch reject fast.
    // force:true skips actionability waits (rAF is frozen under the fake clock);
    // the trailing clicks target the now-vanished "Resend link" locator and are
    // expected to fail fast — that they don't fire is the point.
    await page.context().setOffline(true);
    await activeBtn.click({ force: true });
    await activeBtn.click({ force: true, timeout: 1500 }).catch(() => {});
    await activeBtn.click({ force: true, timeout: 1500 }).catch(() => {});

    // The button flips back to a disabled countdown after the single request.
    await expect(page.getByRole('button', { name: /Resend in 0:\d{2}/ })).toBeVisible();
    expect(resetRequests).toBe(1);
  });

  test('[FE-0160] resend() always sets resentOk even when the request errors (no enumeration leak)', async ({
    page
  }) => {
    await page.clock.install();
    await page.goto(`${FRONTEND}/reset/sent?email=someone@example.com`);

    // Gate the clock on the hydrated countdown, then run it to 0.
    // (page.route interception would break the fake clock, so we force the
    // request to FAIL via offline mode instead — fetch only rejects on network
    // errors, which is exactly the catch the contract exercises.)
    await expect(page.getByRole('button', { name: /Resend in 0:\d{2}/ })).toBeVisible();
    await tickPastCountdown(page);

    const activeBtn = page.getByRole('button', { name: 'Resend link' });
    await expect(activeBtn).toBeEnabled();
    await page.context().setOffline(true);
    await activeBtn.click({ force: true });

    // resend() swallows the network error → resentOk=true, but it also resets
    // secondsLeft=60, so the button first goes back to a disabled countdown.
    await expect(page.getByRole('button', { name: /Resend in 0:\d{2}/ })).toBeVisible();

    // Run the second countdown to 0: with resentOk=true the {:else} branch
    // now renders "Sent again ✓" — proving the catch set resentOk regardless
    // of the failed request (no enumeration leak).
    await tickPastCountdown(page, 'Sent again ✓');
    await expect(page.getByRole('button', { name: 'Sent again ✓' })).toBeVisible();
  });

  test('[FE-0161] reflected ?email HTML entities neutralised by Svelte auto-escape in <h1> and <pre>', async ({
    page
  }) => {
    const payload = '"<img src=x>"';
    await page.goto(`${FRONTEND}/reset/sent?email=${encodeURIComponent(payload)}`);

    // {email} interpolated literally in the headline and the preview.
    const h1 = page.locator('h1.reset-headline');
    await expect(h1).toContainText(payload);
    const preview = page.locator('pre.email-preview');
    await expect(preview).toContainText(payload);

    // No injected <img> materialised from the payload in those elements.
    await expect(h1.locator('img')).toHaveCount(0);
    await expect(preview.locator('img')).toHaveCount(0);
  });

  test('[FE-0162] static email preview shows the literal placeholder token, never a real token', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/reset/sent?email=someone@example.com`);

    const preview = page.locator('pre.email-preview');
    // Literal placeholder, escaped angle brackets render as <your-token>.
    await expect(preview).toContainText('https://astrophoto.pics/reset/<your-token>');
    // No real reset token leaked into the DOM (placeholder only).
    const text = (await preview.textContent()) ?? '';
    expect(text).not.toMatch(/reset\/[A-Za-z0-9_-]{16,}/);
  });
});

// ─────────────────────────────────────────────────────────────────────────
// /email-change/[token]
// ─────────────────────────────────────────────────────────────────────────

test.describe('email-change/[token] edge cases', () => {
  test('[FE-0169] confirm status "taken" → panel-danger "Address already taken" with link to /settings/email', async ({
    page,
    request
  }) => {
    // Reach the 'taken' branch: a valid, unused email_change_tokens row whose
    // new_email is already used by another account → unique-violation on the
    // users.email update → backend returns { status: 'taken' }.
    const ts = Date.now();
    const changer = freshAccount(ts, 'au2'); // user whose email we try to change
    const blocker = freshAccount(ts, 'au2'); // owns the target email already
    await apiSignup(request, changer);
    await apiSignup(request, blocker);

    const token = `tok_${ts}_${Math.random().toString(36).slice(2)}`;
    const hashHex = createHash('sha256').update(token).digest('hex');
    const userId = sql(`select id from users where email = '${changer.email}'`);

    sql(
      `insert into email_change_tokens (token_hash, user_id, new_email, expires_at)
       values (decode('${hashHex}', 'hex'), '${userId}', '${blocker.email}', now() + interval '1 hour')`
    );

    await page.goto(`${FRONTEND}/email-change/${token}`);
    await page.waitForLoadState('networkidle');

    // data.status === 'taken' branch.
    await expect(page.locator('.panel-danger .panel-title')).toHaveText('Address already taken');
    await expect(page.locator('a[href="/settings/email"]')).toBeVisible();
  });
});
