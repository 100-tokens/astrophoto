import { test, expect } from '@playwright/test';
import type { Locator, Page, APIRequestContext } from '@playwright/test';
import { FRONTEND, signupVerifiedAndLogin, sql } from './helpers';

/**
 * Batch: /settings/* front-tagged edge cases.
 *
 * Most pages live under settings/+layout.server.ts which redirects anonymous
 * users to /signin. Each test creates its OWN fresh, verified, signed-in
 * account via signupVerifiedAndLogin and then navigates to the route.
 *
 * NOTE on observability: native `<form method=POST>` actions issue their
 * backend fetch server→server (SvelteKit action), so the browser only sees
 * one POST to the page URL. Browser-side fetches in HandleField/AutosaveField
 * ARE observable and interceptable with page.route.
 */

/** signupVerifiedAndLogin clicks submit on the non-enhanced signin form but
 *  does not await the post-login full navigation to `/`. Wait for it so the
 *  subsequent goto(settings) doesn't race a redirect back to /signin. */
async function loginAndSettle(page: Page, request: APIRequestContext, prefix: string) {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), prefix);
  await page.waitForURL(`${FRONTEND}/`);
  return acc;
}

/**
 * HandleField/AutosaveField inputs are pre-seeded Svelte controlled inputs
 * ($state from a prop). Playwright key events (fill / Backspace) and a single
 * synthetic dispatch race the Svelte re-render: the component's `{value}`
 * binding re-applies its prior state, so the typed text gets spliced into the
 * old value or snaps back. We set the value via the native setter and dispatch
 * a real `input` event — exactly what the component's `oninput` listens for —
 * and retry until `inputValue()` is stable at `text` (the binding has settled).
 */
async function setControlled(input: Locator, text: string): Promise<void> {
  await expect(async () => {
    await input.evaluate((el, value) => {
      const node = el as HTMLInputElement;
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
      setter?.call(node, value);
      node.dispatchEvent(new InputEvent('input', { bubbles: true }));
    }, text);
    await input.page().waitForTimeout(50);
    expect(await input.inputValue()).toBe(text);
  }).toPass({ timeout: 4000 });
}

test.describe('settings profile', () => {
  test('[FE-0223] anonymous hitting /settings/profile is 303-redirected to /signin?next=', async ({
    page
  }) => {
    // settings/+layout.server.ts: if (!locals.user) redirect(303, /signin?next=...)
    await page.goto(`${FRONTEND}/settings/profile`);
    await page.waitForURL(/\/signin\?next=/);
    const url = new URL(page.url());
    expect(url.pathname).toBe('/signin');
    expect(url.searchParams.get('next')).toBe('/settings/profile');
  });

  test('[FE-0221] HandleField: editing handle to the same value (after lowercase/trim) keeps Save disabled and fires no POST', async ({
    page,
    request
  }) => {
    const acc = await loginAndSettle(page, request, 'fe0221');
    await page.goto(`${FRONTEND}/settings/profile`);

    let handlePost = false;
    await page.route('**/api/me/handle', (route) => {
      handlePost = true;
      return route.continue();
    });

    const input = page.getByLabel('Handle');
    const saveBtn = page.getByRole('button', { name: 'Save handle' });

    // Re-type the current handle uppercased — normalized === current.toLowerCase()
    // → dirty=false → avail='current' → canSave=false → button stays disabled.
    await setControlled(input, acc.handle.toUpperCase());
    await expect(saveBtn).toBeDisabled();

    // Give any (non-existent) debounce a window; assert no /api/me/handle POST.
    await page.waitForTimeout(500);
    expect(handlePost).toBe(false);
  });

  test('[FE-0227] HandleField: invalid-format handle ("ab", "Hé!") shows the invalid hint with NO network call', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0227');
    await page.goto(`${FRONTEND}/settings/profile`);

    let checkCall = false;
    let handlePost = false;
    await page.route('**/api/auth/handle-check**', (route) => {
      checkCall = true;
      return route.continue();
    });
    await page.route('**/api/me/handle', (route) => {
      handlePost = true;
      return route.continue();
    });

    const input = page.getByLabel('Handle');
    const invalidMsg = page.locator('.msg.err', {
      hasText: '3–30 chars, lowercase letters / digits / - _'
    });

    // Too short: fails /^[a-z0-9_-]{3,30}$/ → avail='invalid', no fetch.
    await setControlled(input, 'ab');
    await expect(invalidMsg).toBeVisible();

    // Illegal chars: same branch.
    await setControlled(input, 'Hé!');
    await expect(invalidMsg).toBeVisible();

    await page.waitForTimeout(500); // longer than the 350ms availability debounce
    expect(checkCall).toBe(false);
    expect(handlePost).toBe(false);
  });

  test('[FE-0222] AutosaveField: rapid keystrokes (<600ms apart) debounce into a single POST', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0222');
    await page.goto(`${FRONTEND}/settings/profile`);

    // AutosaveField saves to action="" → resolves to the page URL.
    let posts = 0;
    await page.route(`${FRONTEND}/settings/profile`, (route) => {
      if (route.request().method() === 'POST') posts += 1;
      return route.continue();
    });

    const input = page.getByLabel('Display name');
    // Start from a known-empty value (the input is pre-seeded with the name).
    await input.click();
    await input.press('ControlOrMeta+a');
    await input.press('Backspace');
    await page.waitForTimeout(100);
    // Five quick keystrokes well within the 600ms debounce window.
    for (const ch of ['A', 'l', 't', 'a', 'r']) {
      await input.press(ch);
      await page.waitForTimeout(80);
    }
    // Wait past the debounce so the single trailing save fires.
    await page.waitForTimeout(1200);
    expect(posts).toBe(1);
  });

  test('[FE-0226] AutosaveField: a non-ok save response renders "● Save failed — retry" and no Saved state', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0226');
    await page.goto(`${FRONTEND}/settings/profile`);

    // Force the autosave POST (action="" → page URL) to fail.
    await page.route(`${FRONTEND}/settings/profile`, (route) => {
      if (route.request().method() === 'POST') {
        return route.fulfill({ status: 500, body: 'nope' });
      }
      return route.continue();
    });

    const input = page.getByLabel('Display name');
    await setControlled(input, 'Vega Observatory');

    // save() catch → error=true → "● Save failed — retry".
    await expect(page.locator('.autosave .err')).toHaveText('● Save failed — retry');
    await expect(page.locator('.autosave .saved')).toHaveCount(0);
  });
});

test.describe('settings password', () => {
  test('[FE-0200] an 11-char new password short-circuits the action with too_short (server guard)', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0200');
    await page.goto(`${FRONTEND}/settings/password`);

    const newPw = page.locator('#new_password');
    // Clear the client guards so the 11-char value reaches the action,
    // where `if (new_password.length < 12) return fail(400,{error:'too_short'})`.
    await newPw.evaluate((el) => {
      el.removeAttribute('required');
      el.removeAttribute('minlength');
    });
    await page.fill('#current_password', 'whatever12345');
    await newPw.fill('shortpw1234'); // 11 chars
    await page.click('button[type="submit"]');
    await page.waitForLoadState('networkidle'); // non-enhanced form → full POST

    // The rendered too_short message is the observable proof the guard fired
    // (the backend password-change call is server-side and never reached).
    await expect(page.locator('p.err', { hasText: 'Use at least 12 characters.' })).toBeVisible();
  });

  test('[FE-0208] a successful change renders "Password changed. Other devices have been signed out."', async ({
    page,
    request
  }) => {
    const acc = await loginAndSettle(page, request, 'fe0208');
    await page.goto(`${FRONTEND}/settings/password`);

    await page.fill('#current_password', acc.password);
    await page.fill('#new_password', 'astrophoto-changed-pw'); // ≥12, uncommon (low-entropy: not a secret)
    await page.click('button[type="submit"]');
    await page.waitForLoadState('networkidle');

    await expect(
      page.locator('p.ok', { hasText: 'Password changed. Other devices have been signed out.' })
    ).toBeVisible();
  });
});

test.describe('settings email', () => {
  test('[FE-0216] a successful email-change request should show the confirmation message in the modal', async ({
    page,
    request
  }) => {
    const acc = await loginAndSettle(page, request, 'fe0216');
    await page.goto(`${FRONTEND}/settings/email`);

    await page.getByRole('button', { name: 'Change…' }).click();
    const modalEmail = page.locator('#new_email');
    await expect(modalEmail).toBeVisible(); // modal opened (showModal=true)
    await modalEmail.fill(`changed-${Date.now()}@example.com`);
    await page.fill('#current_password', acc.password);
    await page.getByRole('button', { name: 'Send confirmation link' }).click();
    await page.waitForLoadState('networkidle');

    // attendu (FROZEN contract): {#if form?.ok} renders this inside the modal.
    // The form is non-enhanced and showModal is local $state(false): a full
    // POST reload resets showModal=false so <Modal {#if open}> renders nothing,
    // and the message never reaches the DOM. If this is RED it is a real bug —
    // do NOT weaken; assert the contract as written.
    await expect(
      page.locator('p.ok', { hasText: 'Check your new inbox for a confirmation link.' })
    ).toBeVisible();
  });
});

test.describe('settings appearance', () => {
  test('[FE-0229] anonymous setTheme swallows the backend 401, persists the cookie, and 303-redirects', async ({
    request
  }) => {
    // Form actions run BEFORE the layout load, so we POST the action directly
    // from an anonymous (no-session) request context. syncToBackend catches
    // the backend 401 (anonymous), the cookie is set, and the action throws
    // redirect(303). Posted as a fetch-style form action, SvelteKit serialises
    // the redirect into a 200 JSON envelope {type:'redirect',status:303,...};
    // that envelope (not a /signin error) is the proof the 401 was swallowed.
    const res = await request.post(`${FRONTEND}/settings/appearance?/setTheme`, {
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      form: { theme: 'dark' },
      maxRedirects: 0
    });
    expect(res.status()).toBe(200);
    const body = (await res.json()) as { type: string; status: number; location: string };
    expect(body.type).toBe('redirect');
    expect(body.status).toBe(303);
    expect(body.location).toBe('/settings/appearance');
    // The theme cookie persists (maxAge 1 year) even without an account.
    const setCookie = res.headers()['set-cookie'] ?? '';
    expect(setCookie).toContain('theme=dark');
  });

  test('[FE-0230] setting theme then density does not clobber the other column (independent cookies)', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0230');
    await page.goto(`${FRONTEND}/settings/appearance`);

    // Move OFF the defaults first: theme=light (default is dark).
    await page.locator('form[action="?/setTheme"] button[value="light"]').click();
    await page.waitForURL(`${FRONTEND}/settings/appearance`);
    await expect(page.locator('form[action="?/setTheme"] button[value="light"]')).toHaveClass(
      /active/
    );

    // Now set density=data — must NOT reset theme back to dark.
    await page.locator('form[action="?/setDensity"] button[value="data"]').click();
    await page.waitForURL(`${FRONTEND}/settings/appearance`);

    await expect(page.locator('form[action="?/setDensity"] button[value="data"]')).toHaveClass(
      /active/
    );
    // Theme survived the density write (coalesce / independent cookie).
    await expect(page.locator('form[action="?/setTheme"] button[value="light"]')).toHaveClass(
      /active/
    );
  });

  test('[FE-0232] a valid setTheme redirects (303) to /settings/appearance and the active chip follows the new theme', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0232');
    await page.goto(`${FRONTEND}/settings/appearance`);

    // Switch to light, expect a full reload back to the page (the action ends
    // with throw redirect(303,'/settings/appearance')).
    await page.locator('form[action="?/setTheme"] button[value="light"]').click();
    await page.waitForURL(`${FRONTEND}/settings/appearance`);

    // The active chip tracks data.preferences.theme === 'light' now.
    await expect(page.locator('form[action="?/setTheme"] button[value="light"]')).toHaveClass(
      /active/
    );
    await expect(page.locator('form[action="?/setTheme"] button[value="dark"]')).not.toHaveClass(
      /active/
    );
  });

  test('[FE-0233] DENSITY row: the WORK chip gets class:active when density===work, chips are value=work|data in ?/setDensity', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0233');
    await page.goto(`${FRONTEND}/settings/appearance`);

    const densityForm = page.locator('form[action="?/setDensity"]');
    await expect(densityForm.locator('button[name="density"][value="work"]')).toHaveCount(1);
    await expect(densityForm.locator('button[name="density"][value="data"]')).toHaveCount(1);

    // Drive density to 'work' explicitly, then assert WORK is active.
    await densityForm.locator('button[value="work"]').click();
    await page.waitForURL(`${FRONTEND}/settings/appearance`);
    await expect(densityForm.locator('button[value="work"]')).toHaveClass(/active/);
  });
});

test.describe('settings delete', () => {
  test('[FE-0234] a non-exact confirmation phrase keeps the button disabled and the action returns the phrase error', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0234');
    await page.goto(`${FRONTEND}/settings/delete`);

    const phrase = page.locator('#confirmation_phrase');
    const btn = page.getByRole('button', { name: 'Begin 7-day deletion' });

    // Lowercase phrase ≠ 'DELETE MY ACCOUNT' → button stays disabled.
    await phrase.fill('delete my account');
    await expect(btn).toBeDisabled();

    // Strip `disabled` to reach the server guard: phrase !== exact → fail(400,{error:'phrase'}).
    await btn.evaluate((el) => el.removeAttribute('disabled'));
    await btn.click();
    await page.waitForLoadState('networkidle');
    await expect(page.locator('p.err', { hasText: "The phrase doesn't match." })).toBeVisible();
  });

  test('[FE-0239][FE-0240] a successful deletion request reloads into the "DELETION SCHEDULED" panel with cancel form, date, and export link', async ({
    page,
    request
  }) => {
    const acc = await loginAndSettle(page, request, 'fe0239');
    await page.goto(`${FRONTEND}/settings/delete`);

    // Exact phrase + correct password → requestDeletion ok → redirect(303,'/settings/delete').
    await page.fill('#current_password', acc.password);
    await page.fill('#confirmation_phrase', 'DELETE MY ACCOUNT');
    await page.getByRole('button', { name: 'Begin 7-day deletion' }).click();
    await page.waitForURL(`${FRONTEND}/settings/delete`);

    // FE-0239: load reads locals.user.pending_deletion_at → the panel replaces the form.
    await expect(page.locator('.panel.danger .eyebrow')).toHaveText('● DELETION SCHEDULED');
    // FE-0240: cancel form + export download link.
    await expect(
      page.getByRole('button', { name: 'Cancel deletion · keep my account' })
    ).toBeVisible();
    const exportLink = page.getByRole('link', { name: 'Download my archive (JSON)' });
    await expect(exportLink).toHaveAttribute('download', '');
    await expect(exportLink).toHaveAttribute('href', /\/api\/me\/export\.json$/);
    // The form is gone now.
    await expect(page.locator('#confirmation_phrase')).toHaveCount(0);
  });
});

test.describe('settings sessions', () => {
  test('[FE-0248] with only the current session, the "sign out others" block is hidden and the row shows "· this device"', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0248');
    await page.goto(`${FRONTEND}/settings/sessions`);

    // otherCount = sessions.filter(!is_current).length === 0 → block hidden.
    await expect(page.getByRole('button', { name: 'Sign out of all other sessions' })).toHaveCount(
      0
    );
    // The current row marks itself.
    await expect(page.locator('.session-row.current .muted-accent')).toHaveText('· this device');
    // And exposes no Revoke control (only rendered for !s.is_current).
    await expect(page.getByRole('button', { name: /^Revoke session/ })).toHaveCount(0);
  });
});

test.describe('settings tokens', () => {
  test('[FE-0257] with no tokens the list shows the explicit empty state "No tokens yet."', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0257');
    await page.goto(`${FRONTEND}/settings/tokens`);

    await expect(page.locator('li.empty')).toHaveText('No tokens yet.');
  });

  test('[FE-0251] a whitespace-only token name is rejected client-action-side with "Give the token a name."', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0251');
    await page.goto(`${FRONTEND}/settings/tokens`);

    // "   " satisfies HTML required (non-empty) but trims to '' in the action
    // → fail(400,{error:'name_required'}). Form uses use:enhance.
    await page.fill('#name', '   ');
    await page.getByRole('button', { name: 'Generate token' }).click();

    // The rendered name_required message is the observable proof the action
    // short-circuited before api.createApiToken (that backend call is
    // server-side and not browser-observable). No token row appeared.
    await expect(page.locator('p.err', { hasText: 'Give the token a name.' })).toBeVisible();
    await expect(page.locator('li.token-row')).toHaveCount(0);
  });

  test('[FE-0258] a freshly created token reveals the full secret once with the one-time warning; existing rows never show a secret', async ({
    page,
    request
  }) => {
    const acc = await loginAndSettle(page, request, 'fe0258');
    await page.goto(`${FRONTEND}/settings/tokens`);

    await page.fill('#name', 'PixInsight on iMac');
    await page.getByRole('button', { name: 'Generate token' }).click();

    // Reveal block: {#if form && 'created' in form}.
    const secret = page.locator('.reveal code.secret');
    await expect(secret).toBeVisible();
    const secretText = (await secret.textContent())?.trim() ?? '';
    expect(secretText.length).toBeGreaterThan(20);
    await expect(page.locator('.reveal .warn')).toContainText(
      'This is the only time the full token is shown'
    );

    // The persisted row shows only the prefix…, never the full secret.
    const row = page.locator('li.token-row', { hasText: 'PixInsight on iMac' });
    await expect(row).toBeVisible();
    await expect(row.locator('.meta').first()).toContainText('…');
    await expect(row).not.toContainText(secretText);

    // Sanity: only one token row, and the backend stored a hash (no plaintext).
    const hashCount = sql(
      `select count(*) from api_tokens t join users u on u.id=t.user_id where u.email='${acc.email}'`
    );
    expect(Number(hashCount)).toBe(1);
  });

  test('[FE-0253] double-clicking Revoke ends with the row re-rendered as "(revoked)" after invalidateAll', async ({
    page,
    request
  }) => {
    await loginAndSettle(page, request, 'fe0253');
    await page.goto(`${FRONTEND}/settings/tokens`);

    // Seed a token to revoke.
    await page.fill('#name', 'Doomed token');
    await page.getByRole('button', { name: 'Generate token' }).click();
    const row = page.locator('li.token-row', { hasText: 'Doomed token' });
    await expect(row).toBeVisible();

    // Rapid double revoke. The first revokes (204); the second hits the
    // `revoked_at is null` guard → 404 → fail(500) server-side. The observable
    // end-state (use:enhance → invalidateAll) is the row showing "(revoked)".
    const revokeBtn = row.getByRole('button', { name: /^Revoke token/ });
    await revokeBtn.click();
    // Button may detach as the list re-renders; clicking twice fast is the
    // intent — fire a second click best-effort, ignore if already gone.
    await revokeBtn.click({ timeout: 1000 }).catch(() => {});

    await expect(
      page.locator('li.token-row.revoked', { hasText: 'Doomed token' }).locator('.revoked-tag')
    ).toHaveText('(revoked)');
  });
});
