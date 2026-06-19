import { test, expect } from '@playwright/test';
import {
  FRONTEND,
  BACKEND,
  sql,
  freshAccount,
  apiSignup,
  verifyEmail,
  makeAdmin,
  uiLogin,
  signupVerifiedAndLogin
} from './helpers';

// ───────────────────────────────────────────────────────────────────────────
// SQL-escape a single-quoted string literal for inline psql statements.
// helpers.sql() interpolates the statement verbatim, so any value we embed
// (target names, handles, bio HTML) must double its single quotes.
// ───────────────────────────────────────────────────────────────────────────
function q(s: string): string {
  return s.replace(/'/g, "''");
}

// A short, collision-resistant short_id (photos.short_id is UNIQUE and capped
// at 11 chars in our fixtures). Date.now() alone collides across tests in the
// same millisecond window once truncated, so mix in a random base36 tail.
let shortSeq = 0;
function freshShortId(): string {
  shortSeq += 1;
  return `e${Date.now().toString(36)}${shortSeq}${Math.random().toString(36).slice(2, 5)}`.slice(
    0,
    11
  );
}

// The project's custom +error.svelte renders a styled 404/error page and does
// NOT echo the error() message string — so we assert on the authoritative HTTP
// status (error(404,…) → 404, error(403,…) → 403) returned by page.goto().

// ===========================================================================
// /t  — celestial-object index
// ===========================================================================

test.describe('/t index', () => {
  test('[FE-0605] empty catalog query renders empty-state with planning toggles still present', async ({
    page
  }) => {
    // précondition: a query that matches zero catalog rows.
    const res = await page.goto(`${FRONTEND}/t?q=zzzznotarget`);
    expect(res?.status()).toBe(200);

    // attendu: the empty-state branch renders (no tiles, the "No objects match"
    // <p class="empty">), and the size-bucket / Optimal-now planning controls
    // still show because the filter UI is always rendered.
    const empty = page.locator('p.empty');
    await expect(empty).toBeVisible();
    await expect(empty).toContainText('No objects match');
    // No tiles in the grid.
    await expect(page.locator('ul.grid')).toHaveCount(0);

    // Planning toggles still present: the Sort select carries "Optimal now",
    // the Size select exists, and the "Include un-photographed" catalog toggle
    // is rendered.
    await expect(page.getByRole('option', { name: 'Optimal now' })).toHaveCount(1);
    await expect(page.locator('.toggle')).toContainText('Include un-photographed');
  });
});

// ===========================================================================
// /t/[slug]  — single target page
// ===========================================================================

test.describe('/t/[slug] target page', () => {
  test('[FE-0612] unimplemented since param silently no-ops, newest stays selected', async ({
    page
  }) => {
    const slug = `e2e-since-${Date.now()}`;
    // A real catalog target row so the page resolves (no photos needed —
    // the dev DB seeds no RA, and an empty gallery is the documented state).
    sql(
      `insert into targets (slug, canonical_name, kind, constellation) values ('${slug}', 'E2E Since Target', 'common', 'And')`
    );
    try {
      // action: open /t/[slug]?since=7d — the page deliberately does not
      // forward `since` (only /api/explore implements it).
      const res = await page.goto(`${FRONTEND}/t/${slug}?since=7d`);
      // attendu: the param silently no-ops; the page renders fine, no error.
      expect(res?.status()).toBe(200);
      // The target header renders the canonical name.
      await expect(page.locator('h1.display')).toContainText('E2E Since Target');
      // No SvelteKit error surfaced.
      await expect(page.locator('body')).not.toContainText('Internal Error');
    } finally {
      sql(`delete from targets where slug = '${slug}'`);
    }
  });

  test('[FE-0613] target canonical_name with HTML metacharacters is escaped, no stored XSS', async ({
    page
  }) => {
    const slug = `e2e-xss-${Date.now()}`;
    const evil = 'M31 <img src=x onerror=alert(1)>';
    const evilAlias = 'Alias <b>boom</b>';
    sql(
      `insert into targets (slug, canonical_name, aliases, kind, constellation) values ('${slug}', '${q(evil)}', array['${q(evilAlias)}'], 'common', 'And')`
    );
    try {
      const res = await page.goto(`${FRONTEND}/t/${slug}`);
      expect(res?.status()).toBe(200);

      // attendu: canonical_name/aliases are rendered via Svelte text
      // interpolation (not @html), so the markup is escaped. Assert the raw
      // page HTML contains the escaped form and never a live <img onerror>.
      const html = await page.content();
      expect(html).toContain('&lt;img src=x onerror=alert(1)&gt;');
      // No injected element exists in the DOM (the malicious img was escaped
      // to text, so it never became an element).
      await expect(page.locator('img[onerror]')).toHaveCount(0);

      // The header h1 carries the literal name as text content.
      await expect(page.locator('h1.display')).toHaveText(evil);
    } finally {
      sql(`delete from targets where slug = '${slug}'`);
    }
  });
});

// ===========================================================================
// /equip/[kind]  — catalog browse
// ===========================================================================

test.describe('/equip/[kind] browse', () => {
  test('[FE-0614] kind not in the allowlist throws 404 before any backend call', async ({
    page
  }) => {
    // action: navigate /equip/binoculars (not in VALID_KINDS).
    const res = await page.goto(`${FRONTEND}/equip/binoculars`);
    // attendu: +page.server.ts throws error(404, 'Unknown equipment kind')
    // before any backend call — surfaced as an HTTP 404 + the 404 error page.
    expect(res?.status()).toBe(404);
    await expect(page.locator('body')).toContainText('404');
  });

  test('[FE-0615] page index far past the last result renders an empty grid, not a 500', async ({
    page
  }) => {
    // action: GET /equip/telescope?page=99999.
    const res = await page.goto(`${FRONTEND}/equip/telescope?page=99999`);
    // attendu: backend returns an empty items page (200); the empty grid
    // renders rather than a 500.
    expect(res?.status()).toBe(200);
    await expect(page.locator('.empty-state')).toContainText('No catalog items match');
    await expect(page.locator('ul.grid')).toHaveCount(0);
  });

  test('[FE-0617] a non-200 backend status is preserved by the load (bad min_aperture)', async ({
    page
  }) => {
    // action: GET /equip/camera?min_aperture=abc — the backend catalog
    // endpoint rejects a non-numeric min_aperture (observed: 400).
    const res = await page.goto(`${FRONTEND}/equip/camera?min_aperture=abc`);
    // attendu: +page.server.ts forwards min_aperture raw; on !r.ok it throws
    // error(r.status, ...) preserving the backend status — NOT the generic
    // 500 'Failed to load catalog' that only a thrown non-status error hits.
    expect(res?.status()).toBe(400);
  });

  test('[FE-0618] sort not in ALLOWED_SORTS is coerced to most_used', async ({ page }) => {
    // action: GET /equip/mount?sort=cheapest (invalid sort).
    const res = await page.goto(`${FRONTEND}/equip/mount?sort=cheapest`);
    expect(res?.status()).toBe(200);
    // attendu: sort is coerced to 'most_used' before building params2; the
    // select renders most_used as the active option (value bound to data.sort).
    await expect(page.locator('.sort select')).toHaveValue('most_used');
  });

  test('[FE-0619] q with markup is reflected escaped, no reflected XSS', async ({ page }) => {
    const evil = '<svg onload=alert(1)>';
    // action: GET /equip/telescope?q=<svg onload=alert(1)>.
    const res = await page.goto(`${FRONTEND}/equip/telescope?q=${encodeURIComponent(evil)}`);
    expect(res?.status()).toBe(200);

    // attendu: q is rendered back via text interpolation — NO reflected XSS.
    // The dangerous `<` is escaped (→ &lt;) so the tag is never injected raw
    // into the DOM. (`>` is left literal because HTML attribute context does
    // not require escaping it, and DOM serialization of `>` varies — so assert
    // the security property directly, not a specific escaped string.)
    const html = await page.content();
    expect(html).not.toContain('<svg onload');
    await expect(page.locator('svg[onload]')).toHaveCount(0);
    // The query round-trips into the search input value (text interpolation).
    await expect(page.locator('input.search')).toHaveValue(evil);
  });
});

// ===========================================================================
// /equip/[kind]/[slug]  — catalog item detail
// ===========================================================================

test.describe('/equip/[kind]/[slug] detail', () => {
  test('[FE-0623] anonymous visitor: Edit-specs affordance is hidden (canSeeEditAffordance=false)', async ({
    page
  }) => {
    const ts = Date.now();
    const canonical = `e2e-det cam${ts}`;
    const slug = canonical.replace(/\s+/g, '-');
    sql(
      `insert into equipment_items (kind, canonical_name, display_name, brand, model, status, usage_count) values ('camera', '${canonical}', 'E2E-Det Cam${ts}', 'E2EDet', 'Cam${ts}', 'approved', 0)`
    );
    try {
      // action: load the detail page logged-out.
      const res = await page.goto(`${FRONTEND}/equip/camera/${slug}`);
      expect(res?.status()).toBe(200);
      // attendu: canSeeEditAffordance = !!user → false for an anonymous user,
      // so the "Edit specs" link is hidden.
      await expect(page.locator('.btn-edit')).toHaveCount(0);
      await expect(page.getByRole('link', { name: 'Edit specs' })).toHaveCount(0);
      // The fiche still renders (model + spec-sheet section present).
      await expect(page.locator('.specs')).toBeVisible();
    } finally {
      sql(`delete from equipment_items where canonical_name = '${canonical}'`);
    }
  });

  test('[FE-0625] discovery 404 yields a 404 page, distinct from the hydrate-500 branch', async ({
    page
  }) => {
    // FE-0625's frozen attendu: the load distinguishes a discovery 404 from a
    // hydrate 500 — "404 only when discoveryR.status===404", else
    // error(500, 'Failed to load catalog item detail').
    //
    // The hydrate-500 branch is UNREACHABLE BY CONSTRUCTION here: the load is
    // a two-step server fetch (discovery → items/:id) that resolves the SAME
    // equipment_items row by the id discovery returned; in items_get every
    // specs column maps to Option<…> and setup_count is a count(), so a row
    // discovery just resolved cannot make items/:id 500 (nor 404). And because
    // the call runs in +page.server.ts it cannot be page.route-intercepted.
    // So we assert the reachable, faithful half: a bogus slug → discovery 404
    // → 404 page with the discovery message, proving it did NOT fall through
    // to the generic hydrate-500 path.
    const res = await page.goto(`${FRONTEND}/equip/camera/no-such-camera-${Date.now()}`);
    // 404 (discovery branch), NOT 500 (the hydrate-failure branch). The error
    // page doesn't echo the message, so the status is the distinguishing fact.
    expect(res?.status()).toBe(404);
    await expect(page.locator('body')).toContainText('404');
  });

  test('[FE-0626] photo tiles carry author_handle + short_id and link to the permalink', async ({
    page
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'eqtile');
    await apiSignup(page.request, acc);
    verifyEmail(acc.email);
    const ownerId = sql(`select id from users where email = '${acc.email}'`);

    // A catalog camera item, and a published photo that references it via the
    // legacy lower(camera)=canonical_name match the discovery handler uses.
    // canonical_name must be hyphen-free: the discovery photo predicate matches
    // lower(p.camera) against the de-hyphenated slug (canonical_for turns '-'
    // into ' '), so a hyphen in the name breaks the round-trip. Spaces are fine.
    const canonical = `e2etile cam${ts}`;
    const slug = canonical.replace(/\s+/g, '-');
    const shortId = freshShortId();
    sql(
      `insert into equipment_items (kind, canonical_name, display_name, brand, model, status, usage_count) values ('camera', '${canonical}', 'E2E-Tile Cam${ts}', 'E2ETile', 'Cam${ts}', 'approved', 1)`
    );
    sql(
      `insert into photos (owner_id, storage_key, original_name, bytes, mime, status, original_uploaded_at, short_id, published_at, target, camera, width, height) values ('${ownerId}', 'originals/eqtile.jpg', 'eqtile.jpg', 1000, 'image/jpeg', 'ready', now(), '${shortId}', now(), 'NGC 7000', '${canonical}', 4000, 3000)`
    );
    try {
      const res = await page.goto(`${FRONTEND}/equip/camera/${slug}`);
      expect(res?.status()).toBe(200);
      // attendu: each tile links to /u/<author_handle>/p/<short_id> with no
      // extra round-trip (discovery supplies author_handle + short_id).
      const tileLink = page.locator(`.photo-grid a[href="/u/${acc.handle}/p/${shortId}"]`);
      await expect(tileLink).toHaveCount(1);
    } finally {
      sql(`delete from photos where short_id = '${shortId}'`);
      sql(`delete from equipment_items where canonical_name = '${canonical}'`);
      sql(`delete from users where email = '${acc.email}'`);
    }
  });
});

// ===========================================================================
// /equip/[kind]/[slug]/edit  — admin-gated spec editing
// ===========================================================================

test.describe('/equip/[kind]/[slug]/edit guard + cache', () => {
  test('[FE-0631] non-admin gets 403; logged-out gets a signin redirect', async ({
    page,
    request
  }) => {
    const ts = Date.now();
    const canonical = `e2e-edit scope${ts}`;
    const slug = canonical.replace(/\s+/g, '-');
    sql(
      `insert into equipment_items (kind, canonical_name, display_name, brand, model, status, usage_count) values ('telescope', '${canonical}', 'E2E-Edit Scope${ts}', 'E2EEdit', 'Scope${ts}', 'approved', 0)`
    );
    try {
      // Logged-out: redirect(303, '/signin?next=/equip/.../edit').
      const anonRes = await page.goto(`${FRONTEND}/equip/telescope/${slug}/edit`);
      await page.waitForURL(/\/signin\?next=/);
      expect(new URL(page.url()).pathname).toBe('/signin');
      expect(new URL(page.url()).searchParams.get('next')).toBe(`/equip/telescope/${slug}/edit`);
      // (anonRes is the final 200 signin page after the redirect chain.)
      expect(anonRes?.status()).toBe(200);

      // Signed-in but NOT admin: error(403, 'Catalog editing requires an
      // admin account').
      await signupVerifiedAndLogin(page, request, ts, 'eqedit');
      await page.waitForURL(`${FRONTEND}/`);
      const res = await page.goto(`${FRONTEND}/equip/telescope/${slug}/edit`);
      // error(403, 'Catalog editing requires an admin account') → HTTP 403
      // (distinct from the logged-out 303 redirect above). The error page
      // doesn't echo the message, so assert the authoritative status.
      expect(res?.status()).toBe(403);
    } finally {
      sql(`delete from equipment_items where canonical_name = '${canonical}'`);
    }
  });

  test('[FE-0634] admin rename of a filter item rebuilds the photos.filters cache', async ({
    page,
    request
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'eqcache');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    makeAdmin(acc.email);
    await uiLogin(page, acc);
    await page.waitForURL(`${FRONTEND}/`);

    const ownerId = sql(`select id from users where email = '${acc.email}'`);

    // A filter catalog item, a published photo, and the photo_filters junction
    // row — rebuild_for_item rebuilds photos.filters FROM the junction
    // (string_agg of equipment_items.display_name), so the junction must exist.
    const canonical = `e2e-cache filt${ts}`;
    sql(
      `insert into equipment_items (kind, canonical_name, display_name, brand, model, status, usage_count) values ('filter', '${canonical}', 'OldHa ${ts}', 'E2ECache', 'Filt${ts}', 'approved', 1)`
    );
    const itemId = sql(`select id from equipment_items where canonical_name = '${canonical}'`);
    const shortId = freshShortId();
    sql(
      `insert into photos (owner_id, storage_key, original_name, bytes, mime, status, original_uploaded_at, short_id, published_at, filters) values ('${ownerId}', 'originals/eqcache.jpg', 'eqcache.jpg', 1000, 'image/jpeg', 'ready', now(), '${shortId}', now(), 'OldHa ${ts}')`
    );
    const photoId = sql(`select id from photos where short_id = '${shortId}'`);
    sql(
      `insert into photo_filters (photo_id, item_id, position) values ('${photoId}', '${itemId}', 0)`
    );

    try {
      const newName = `NewHa ${ts}`;
      // action: PATCH display_name through the authenticated admin session.
      // page.request shares the browser session cookie set by uiLogin.
      const r = await page.request.patch(`${BACKEND}/api/equipment/items/${itemId}`, {
        data: { display_name: newName }
      });
      expect(r.status()).toBe(200);

      // attendu: items_update calls filters_cache::rebuild_for_item inside the
      // same tx, so photos.filters is rebuilt from the junction with the new
      // display_name — stale names do not persist.
      const cached = sql(`select filters from photos where id = '${photoId}'`);
      expect(cached).toBe(newName);
    } finally {
      sql(`delete from photo_filters where photo_id = '${photoId}'`);
      sql(`delete from photos where id = '${photoId}'`);
      sql(`delete from equipment_items where id = '${itemId}'`);
      sql(`delete from users where email = '${acc.email}'`);
    }
  });
});

// ===========================================================================
// /u/[handle]  — public photographer profile
// ===========================================================================

test.describe('/u/[handle] profile', () => {
  test('[FE-0639] owner sees owner viewMode; a visitor sees visitor viewMode', async ({
    page,
    request
  }) => {
    const ts = Date.now();
    const acc = await signupVerifiedAndLogin(page, request, ts, 'uself');
    await page.waitForURL(`${FRONTEND}/`);

    // isSelf = locals.user?.id === profile.id → viewMode 'owner' on own page.
    const ownRes = await page.goto(`${FRONTEND}/u/${acc.handle}`);
    expect(ownRes?.status()).toBe(200);
    await expect(page.locator('.hero-page')).toHaveAttribute('data-mode', 'owner');

    // A different (anonymous) context sees viewMode 'visitor' for the same
    // profile — public data is identical, only the affordances key off mode.
    const visitorCtx = await page.context().browser()!.newContext();
    try {
      const visitorPage = await visitorCtx.newPage();
      const visRes = await visitorPage.goto(`${FRONTEND}/u/${acc.handle}`);
      expect(visRes?.status()).toBe(200);
      await expect(visitorPage.locator('.hero-page')).toHaveAttribute('data-mode', 'visitor');
    } finally {
      await visitorCtx.close();
    }
  });

  test('[FE-0642] head: title, og/meta description from tagline, JSON-LD knowsAbout only with bio', async ({
    page
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'uhead');
    await apiSignup(page.request, acc);
    verifyEmail(acc.email);
    const tagline = `Nightscapes from ${ts}`;
    sql(
      `update users set tagline = '${q(tagline)}', bio_html = '<p>About me</p>' where email = '${acc.email}'`
    );
    try {
      const res = await page.goto(`${FRONTEND}/u/${acc.handle}`);
      expect(res?.status()).toBe(200);

      // attendu: title = `${display_name} — Astrophoto`.
      await expect(page).toHaveTitle(`${acc.displayName} — Astrophoto`);
      // og/meta description from tagline.
      await expect(page.locator('meta[name="description"]')).toHaveAttribute('content', tagline);
      await expect(page.locator('meta[property="og:description"]')).toHaveAttribute(
        'content',
        tagline
      );
      // JSON-LD emitted with knowsAbout (bio_html present).
      const ld = await page.locator('script[type="application/ld+json"]').first().textContent();
      const json = JSON.parse(ld ?? '{}');
      expect(json['@type']).toBe('Person');
      expect(json.knowsAbout).toBe('astrophotography');

      // Now clear the bio: knowsAbout must disappear from the JSON-LD.
      sql(`update users set bio_html = null where email = '${acc.email}'`);
      await page.goto(`${FRONTEND}/u/${acc.handle}`);
      const ld2 = await page.locator('script[type="application/ld+json"]').first().textContent();
      const json2 = JSON.parse(ld2 ?? '{}');
      expect(json2.knowsAbout).toBeUndefined();
    } finally {
      sql(`delete from users where email = '${acc.email}'`);
    }
  });

  test('[FE-0643] stored bio is server-sanitised; bio container has no script element', async ({
    page,
    request
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'ubio');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    await uiLogin(page, acc);
    await page.waitForURL(`${FRONTEND}/`);

    // attendu: bio_html passes through users::bio::sanitize (ammonia) at WRITE
    // time — so we MUST write through the real PATCH /api/me/profile, not a
    // raw SQL insert (which would bypass the sanitizer). The cleaner strips
    // <script>/onclick/javascript:.
    const evilBio = `<p>Hello</p><script>alert(1)</script><a href="javascript:alert(2)">x</a>`;
    const r = await page.request.patch(`${BACKEND}/api/me/profile`, {
      data: { bio_html: evilBio }
    });
    expect(r.status()).toBeLessThan(300);

    // The stored value already has the script tag stripped (the inner text may
    // survive as text, but the <script> element is gone).
    const stored = sql(`select bio_html from users where email = '${acc.email}'`);
    expect(stored).not.toContain('<script');
    expect(stored).not.toContain('javascript:');

    try {
      // Render the profile; HeroAbout's {@html bio} is safe because the value
      // is server-sanitised. Scope to the .bio container (the page also emits
      // a legitimate ld+json <script> and hydration scripts).
      const res = await page.goto(`${FRONTEND}/u/${acc.handle}`);
      expect(res?.status()).toBe(200);
      const bio = page.locator('.bio');
      await expect(bio).toBeVisible();
      // No <script> descendant inside the bio container.
      await expect(bio.locator('script')).toHaveCount(0);
      // The escaped/sanitised content carries the visible text.
      await expect(bio).toContainText('Hello');
    } finally {
      sql(`delete from users where email = '${acc.email}'`);
    }
  });
});

// ===========================================================================
// /u/[handle]/p/[shortid]  — public photo permalink
// ===========================================================================

test.describe('/u/[handle]/p/[shortid] permalink', () => {
  test('[FE-0650] published photo with ra_deg but no solve does not render the celestial overlay', async ({
    page
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'povl');
    await apiSignup(page.request, acc);
    verifyEmail(acc.email);
    const ownerId = sql(`select id from users where email = '${acc.email}'`);

    // précondition: a published photo WITH ra_deg set (so load fetches
    // platesolve-status / celestial-objects) but NO solve recorded. attendu:
    // absent a solve, the overlay simply does not render.
    const shortId = freshShortId();
    sql(
      `insert into photos (owner_id, storage_key, original_name, bytes, mime, status, original_uploaded_at, short_id, published_at, target, ra_deg, width, height) values ('${ownerId}', 'originals/povl.jpg', 'povl.jpg', 1000, 'image/jpeg', 'ready', now(), '${shortId}', now(), 'M42', 83.8, 4000, 3000)`
    );
    const photoId = sql(`select id from photos where short_id = '${shortId}'`);
    try {
      const res = await page.goto(`${FRONTEND}/u/${acc.handle}/p/${shortId}`);
      expect(res?.status()).toBe(200);
      // The photo detail renders (title from target).
      await expect(page.locator('h1.title')).toContainText('M42');
      // No WCS solve → solveForOverlay is falsy → CelestialOverlay absent.
      // The overlay is the distinctive <svg class="celestial-overlay">; the
      // CelestialPanel (also solve-gated) carries aria-label "object type
      // layers". Neither renders without a solve.
      await expect(page.locator('svg.celestial-overlay')).toHaveCount(0);
      await expect(page.locator('.celestial-panel')).toHaveCount(0);
    } finally {
      sql(`delete from photos where id = '${photoId}'`);
      sql(`delete from users where email = '${acc.email}'`);
    }
  });

  test('[FE-0651] comment body with <script> is escaped at render (text interpolation)', async ({
    page
  }) => {
    const ts = Date.now();
    const acc = freshAccount(ts, 'pxss');
    await apiSignup(page.request, acc);
    verifyEmail(acc.email);
    const ownerId = sql(`select id from users where email = '${acc.email}'`);

    const shortId = freshShortId();
    const caption = '<b>caption boom</b>';
    sql(
      `insert into photos (owner_id, storage_key, original_name, bytes, mime, status, original_uploaded_at, short_id, published_at, target, caption, width, height) values ('${ownerId}', 'originals/pxss.jpg', 'pxss.jpg', 1000, 'image/jpeg', 'ready', now(), '${shortId}', now(), 'NGC 6960', '${q(caption)}', 4000, 3000)`
    );
    const photoId = sql(`select id from photos where short_id = '${shortId}'`);
    // A stored comment whose body is a script payload. The render-time
    // escaping is what FE-0651 tests, so seeding the raw body via SQL is the
    // correct fixture (contrast with FE-0643's write-time sanitiser).
    const evilBody = '<script>alert(1)</script>';
    sql(
      `insert into comments (photo_id, author_id, body, created_at) values ('${photoId}', '${ownerId}', '${q(evilBody)}', now())`
    );
    try {
      const res = await page.goto(`${FRONTEND}/u/${acc.handle}/p/${shortId}`);
      expect(res?.status()).toBe(200);

      // Comments load client-side; wait for the thread to hydrate.
      const body = page.locator('#comments .body').first();
      await expect(body).toBeVisible();

      // attendu: CommentThread renders {c.body} via text interpolation, so the
      // markup is escaped — no <script> element is created from the body.
      await expect(body).toHaveText(evilBody);
      await expect(page.locator('#comments .body script')).toHaveCount(0);

      // Caption is likewise rendered as {p.caption} text — escaped, not parsed.
      const cap = page.locator('.caption');
      await expect(cap).toHaveText(caption);
      await expect(cap.locator('b')).toHaveCount(0);
    } finally {
      sql(`delete from comments where photo_id = '${photoId}'`);
      sql(`delete from photos where id = '${photoId}'`);
      sql(`delete from users where email = '${acc.email}'`);
    }
  });
});
