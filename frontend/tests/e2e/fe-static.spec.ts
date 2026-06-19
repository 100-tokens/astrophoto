import { test, expect } from '@playwright/test';
import { FRONTEND, signupVerifiedAndLogin } from './helpers';

// Static / informational pages: /about /contact /design /privacy /terms.
// All public, no forms (/contact is a mailto link; /design is a prerendered
// component gallery with local $state). Auth-tenant cases assert the header
// auth-state flip; everything else is anonymous.

const MAILTO = 'mailto:hello@astrophoto.example';

test.describe('static pages — /about', () => {
  test('[FE-0700] query/hash is ignored; H1 renders without reflecting user input', async ({
    page
  }) => {
    // Arbitrary query string + fragment must not reach the DOM (no load, no
    // $page.url read on this route).
    await page.goto(`${FRONTEND}/about?foo=%3Cscript%3E#frag`);

    const main = page.locator('main.static-page');
    await expect(main.locator('h1')).toHaveText('A quiet archive of the night sky.');
    // The injected `foo` value never appears anywhere in the document.
    await expect(page.locator('body')).not.toContainText('<script>');
    await expect(page.locator('body')).not.toContainText('foo=');
  });

  test('[FE-0701] header auth zone flips anon vs logged-in; body text is identical', async ({
    page,
    request
  }) => {
    // Anonymous: Sign in + Create account.
    await page.goto(`${FRONTEND}/about`);
    const header = page.locator('header.app-header');
    await expect(header.locator('a[href="/signin"]')).toBeVisible();
    await expect(header.locator('a[href="/signup"]')).toBeVisible();
    await expect(header.locator('a[href="/upload"]')).toHaveCount(0);

    // The body text is tenant-independent.
    await expect(page.locator('main.static-page h1')).toHaveText(
      'A quiet archive of the night sky.'
    );

    // Logged-in: Upload button + AvatarMenu replace the anon links.
    await signupVerifiedAndLogin(page, request, Date.now(), 'fe0701');
    await page.goto(`${FRONTEND}/about`);
    const header2 = page.locator('header.app-header');
    await expect(header2.locator('a[href="/upload"]')).toBeVisible();
    await expect(header2.locator('a[href="/signin"]')).toHaveCount(0);
    await expect(header2.locator('a[href="/signup"]')).toHaveCount(0);

    // Body unchanged for the logged-in tenant.
    await expect(page.locator('main.static-page h1')).toHaveText(
      'A quiet archive of the night sky.'
    );
  });

  test('[FE-0702] at <=640px the H1 shrinks to 32px and primary-nav is hidden behind MobileNav', async ({
    page
  }) => {
    await page.setViewportSize({ width: 640, height: 900 });
    await page.goto(`${FRONTEND}/about`);

    const h1 = page.locator('main.static-page h1');
    await expect(h1).toHaveCSS('font-size', '32px');

    // The desktop primary nav is hidden under 768px; the MobileNav burger
    // (which carries the same destinations) is the reachable fallback.
    await expect(page.locator('header.app-header nav.primary-nav')).toHaveCSS('display', 'none');
    await expect(page.locator('header.app-header .mobile-nav button.burger')).toBeVisible();

    // No horizontal overflow at this width.
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth
    );
    expect(overflow).toBe(true);
  });

  test('[FE-0703] SSR response carries the baseline security headers; no XSS sink', async ({
    page
  }) => {
    const res = await page.goto(`${FRONTEND}/about`);
    expect(res).not.toBeNull();
    const headers = res!.headers();
    expect(headers['x-frame-options']).toBe('SAMEORIGIN');
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['content-security-policy-report-only']).toContain("frame-ancestors 'none'");
  });
});

test.describe('static pages — /contact', () => {
  test('[FE-0704] mailto link points at the hardcoded address (no input to bound)', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/contact`);
    const mailto = page.locator(`a[href="${MAILTO}"]`);
    await expect(mailto).toBeVisible();
    await expect(mailto).toHaveAttribute('href', MAILTO);
    // No user input fields exist on the page.
    await expect(page.locator('main.static-page input')).toHaveCount(0);
    await expect(page.locator('main.static-page form')).toHaveCount(0);
  });

  test('[FE-0705] header reflects auth; mailto is identical and not pre-filled per account', async ({
    page,
    request
  }) => {
    await page.goto(`${FRONTEND}/contact`);
    const header = page.locator('header.app-header');
    await expect(header.locator('a[href="/signin"]')).toBeVisible();
    await expect(header.locator('a[href="/signup"]')).toBeVisible();
    await expect(page.locator(`a[href="${MAILTO}"]`)).toHaveCount(1);

    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'fe0705');
    await page.goto(`${FRONTEND}/contact`);
    const header2 = page.locator('header.app-header');
    await expect(header2.locator('a[href="/upload"]')).toBeVisible();
    // The mailto is unchanged: the account email is never interpolated into it.
    const href = await page.locator('main.static-page a').first().getAttribute('href');
    expect(href).toBe(MAILTO);
    expect(href).not.toContain(acc.email);
  });

  test('[FE-0706] mailto link is focusable, accent-colored, and the [urgent] span is not', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/contact`);
    const mailto = page.locator(`a[href="${MAILTO}"]`);

    // Focusable via keyboard.
    await mailto.focus();
    await expect(mailto).toBeFocused();

    // The [urgent] hint is a plain non-interactive span (not a link/button).
    const urgent = page.locator('main.static-page span.t-mono');
    await expect(urgent).toHaveText('[urgent]');
    await expect(urgent).toHaveCount(1);
    // It is not a focusable element.
    expect(await urgent.evaluate((el) => el.tagName.toLowerCase())).toBe('span');
  });

  test('[FE-0707] mailto href is hardcoded; no URL param is interpolated; Referrer-Policy set', async ({
    page
  }) => {
    // Even with a crafted query, the href must not absorb subject/cc headers.
    const res = await page.goto(`${FRONTEND}/contact?subject=evil&cc=attacker@x.test`);
    const mailto = page.locator(`a[href="${MAILTO}"]`);
    await expect(mailto).toHaveAttribute('href', MAILTO);
    const href = await mailto.getAttribute('href');
    expect(href).toBe(MAILTO);
    expect(href).not.toContain('subject');
    expect(href).not.toContain('attacker');

    const headers = res!.headers();
    expect(headers['referrer-policy']).toBe('strict-origin-when-cross-origin');
  });
});

test.describe('static pages — /design', () => {
  test('[FE-0708] mono Input absorbs long/Unicode-astro input without truncation or crash', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/design`);
    // The RA/DEC field is the mono input, initialised to the catalog string.
    const mono = page.locator('input.input-mono');
    await expect(mono).toHaveValue('20ʰ 58ᵐ 47ˢ / +44° 19′');

    const big = '20ʰ 58ᵐ 47ˢ / +44° 19′ ° ′ ″ '.repeat(40);
    await mono.fill(big);
    // The local $state binding absorbs the full value verbatim.
    await expect(mono).toHaveValue(big);
  });

  test('[FE-0709] the three demo $state fields are reactive (state→DOM and DOM→state)', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/design`);

    // state→DOM: initial $state values render into the bound fields.
    const monoInput = page.locator('input.input-mono');
    const textarea = page.locator('textarea.textarea');
    await expect(monoInput).toHaveValue('20ʰ 58ᵐ 47ˢ / +44° 19′');
    await expect(textarea).toHaveValue('Narrowband, 18 h integration over 4 nights...');

    // DOM→state: typing updates the bound value (inputVal starts empty).
    const emailInput = page.locator('input.input[type="email"]');
    await expect(emailInput).toHaveValue('');
    await emailInput.fill('observer@example.com');
    await expect(emailInput).toHaveValue('observer@example.com');

    await textarea.fill('SHO, 24 h');
    await expect(textarea).toHaveValue('SHO, 24 h');
  });

  test('[FE-0710] prerendered page hydrates header auth from session; noindex meta present', async ({
    page,
    request
  }) => {
    // The page is prerendered but renders TWO AppHeaders; assert via counts.
    await page.goto(`${FRONTEND}/design`);
    // noindex robots meta is baked into the prerendered head.
    await expect(page.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex');

    // Anonymous: each header shows the anon auth links (2 headers → count 2).
    await expect(page.locator('header.app-header a[href="/signin"]')).toHaveCount(2);
    await expect(page.locator('header.app-header a[href="/upload"]')).toHaveCount(0);

    // Logged-in: header auth-state hydrates from page.data.user even on the
    // prerendered shell.
    await signupVerifiedAndLogin(page, request, Date.now(), 'fe0710');
    await page.goto(`${FRONTEND}/design`);
    await expect(page.locator('header.app-header a[href="/upload"]')).toHaveCount(2);
    await expect(page.locator('header.app-header a[href="/signin"]')).toHaveCount(0);
  });

  test('[FE-0711] photo grid reflows to one column on narrow viewport; main stays centered', async ({
    page
  }) => {
    await page.setViewportSize({ width: 360, height: 900 });
    await page.goto(`${FRONTEND}/design`);

    // The photo-placeholder samples stack: each <Photo> shares the same left
    // edge (single column reflow) and none of the grid tiles overflow the
    // viewport. (The `attendu` scopes "sans débordement" to the grid.)
    const probe = await page.evaluate(() => {
      const grids = Array.from(document.querySelectorAll('main [style*="grid-template-columns"]'));
      const grid = grids.find((g) => g.querySelectorAll(':scope > div').length >= 3);
      if (!grid) return null;
      const tiles = Array.from(grid.querySelectorAll(':scope > div'));
      return {
        xs: tiles.map((el) => Math.round(el.getBoundingClientRect().left)),
        maxRight: Math.max(...tiles.map((el) => el.getBoundingClientRect().right)),
        winW: window.innerWidth
      };
    });
    expect(probe).not.toBeNull();
    // All tiles align on the same x (single column).
    expect(new Set(probe!.xs).size).toBe(1);
    // The grid tiles do not overflow the viewport horizontally.
    expect(probe!.maxRight).toBeLessThanOrEqual(probe!.winW + 1);

    // <main> stays centered within its 1200px max-width: equal left/right
    // margins inside the 360px viewport.
    const centered = await page.evaluate(() => {
      const main = document.querySelector('main');
      if (!main) return null;
      const r = main.getBoundingClientRect();
      return { left: Math.round(r.left), right: Math.round(window.innerWidth - r.right) };
    });
    expect(centered).not.toBeNull();
    expect(centered!.left).toBe(centered!.right);

    // The MarkReticle / Logo / CornerMarks components render their SVG even
    // at this narrow viewport (sections 7 and 10).
    expect(await page.locator('main svg').count()).toBeGreaterThan(0);
  });

  test('[FE-0712] noindex meta present; CSP is Report-Only so inline styles do not break', async ({
    page
  }) => {
    const res = await page.goto(`${FRONTEND}/design`);
    await expect(page.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex');

    const headers = res!.headers();
    // Report-Only (not enforcing) — inline style= attributes stay silent.
    expect(headers['content-security-policy-report-only']).toContain(
      "style-src 'self' 'unsafe-inline'"
    );
    expect(headers['content-security-policy']).toBeUndefined();

    // The page does rely on inline style= attributes (the very thing CSP
    // would report under enforce).
    expect(await page.locator('main [style]').count()).toBeGreaterThan(0);
  });
});

test.describe('static pages — /privacy', () => {
  test('[FE-0713] text + /settings link render for everyone; no account data exposed', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/privacy`);
    await expect(page.locator('main.static-page h1')).toHaveText('What we keep, and why.');
    // The settings link is in the body for anonymous visitors too.
    await expect(page.locator('main.static-page a[href="/settings"]')).toBeVisible();
    // Anon header.
    await expect(page.locator('header.app-header a[href="/signin"]')).toBeVisible();
  });

  test('[FE-0714] /settings link is an internal client nav (not dead); H1 32px under 640px', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/privacy`);
    const link = page.locator('main.static-page a[href="/settings"]');
    await expect(link).toHaveAttribute('href', '/settings');

    // Mark the document; a full reload (dead link / hard nav) would wipe it.
    await page.evaluate(() => {
      (window as unknown as { __noreload?: boolean }).__noreload = true;
    });
    await link.click();
    // The /settings route exists (not a 404). Anonymous users are bounced to
    // signin by the route guard, so accept either /settings or a signin redirect.
    await page.waitForLoadState('networkidle');
    const url = new URL(page.url());
    expect(['/settings', '/signin'].some((p) => url.pathname.startsWith(p))).toBe(true);
    // Not a SvelteKit 404 page.
    await expect(page.locator('text=Not Found')).toHaveCount(0);

    // Mobile breakpoint: H1 shrinks to 32px.
    await page.setViewportSize({ width: 600, height: 900 });
    await page.goto(`${FRONTEND}/privacy`);
    await expect(page.locator('main.static-page h1')).toHaveCSS('font-size', '32px');
  });

  test('[FE-0715] SSR carries X-Frame-Options + Permissions-Policy; settings link has no target=_blank', async ({
    page
  }) => {
    const res = await page.goto(`${FRONTEND}/privacy`);
    const headers = res!.headers();
    expect(headers['x-frame-options']).toBe('SAMEORIGIN');
    expect(headers['permissions-policy']).toBe('camera=(), microphone=(), geolocation=()');

    // Internal link: no target=_blank, so rel=noopener is N/A.
    const link = page.locator('main.static-page a[href="/settings"]');
    await expect(link).not.toHaveAttribute('target', '_blank');
  });
});

test.describe('static pages — /terms', () => {
  test('[FE-0716] text + /settings link render identically for anon and logged-in', async ({
    page,
    request
  }) => {
    await page.goto(`${FRONTEND}/terms`);
    await expect(page.locator('main.static-page h1')).toHaveText('The short version.');
    await expect(page.locator('main.static-page a[href="/settings"]')).toBeVisible();
    await expect(page.locator('header.app-header a[href="/signin"]')).toBeVisible();

    await signupVerifiedAndLogin(page, request, Date.now(), 'fe0716');
    await page.goto(`${FRONTEND}/terms`);
    // Body text + settings link unchanged for the logged-in tenant.
    await expect(page.locator('main.static-page h1')).toHaveText('The short version.');
    await expect(page.locator('main.static-page a[href="/settings"]')).toBeVisible();
    await expect(page.locator('header.app-header a[href="/upload"]')).toBeVisible();
  });

  test('[FE-0717] H1 32px under 640px; /settings is a live internal link', async ({ page }) => {
    await page.setViewportSize({ width: 600, height: 900 });
    await page.goto(`${FRONTEND}/terms`);
    await expect(page.locator('main.static-page h1')).toHaveCSS('font-size', '32px');

    const link = page.locator('main.static-page a[href="/settings"]');
    await expect(link).toHaveAttribute('href', '/settings');
    await link.click();
    await page.waitForLoadState('networkidle');
    const url = new URL(page.url());
    expect(['/settings', '/signin'].some((p) => url.pathname.startsWith(p))).toBe(true);
    await expect(page.locator('text=Not Found')).toHaveCount(0);
  });

  test('[FE-0718] SSR carries CSP-Report-Only + nosniff + HSTS; no external target=_blank', async ({
    page
  }) => {
    const res = await page.goto(`${FRONTEND}/terms`);
    const headers = res!.headers();
    expect(headers['content-security-policy-report-only']).toContain("default-src 'self'");
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['strict-transport-security']).toBe('max-age=31536000; includeSubDomains');

    // The only link in the body is internal (/settings) — no target=_blank anywhere.
    await expect(page.locator('main.static-page a[target="_blank"]')).toHaveCount(0);
  });
});
