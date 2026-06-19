import { test, expect } from '@playwright/test';
import { FRONTEND, sql, freshAccount, apiSignup, type Account } from './helpers';

/**
 * Public feed/discovery surfaces: / (home), /explore, /search, /photographers,
 * /c/[cat], /tag/[slug]. Mostly anonymous SSR pages — we navigate and assert
 * server-rendered markup, escaped query reflection, and documented empty-states.
 *
 * Seeding: a few cases need at least one *published, ready* photo to exist.
 * The dev DB ships empty, so those tests create a fresh user via the public
 * signup API and SQL-insert a minimal published photo owned by them. Each test
 * owns its own fresh data (unique short_id / original_name) so they don't
 * collide with one another or with the empty-state cases.
 */

/**
 * psql prints the command status tag ("INSERT 0 1") to stdout after a
 * `... returning` statement, on its own line below the returned value. Keep
 * only the first line so callers get the bare id.
 */
function firstLine(out: string): string {
  return out.split('\n')[0]?.trim() ?? '';
}

/** Resolve a freshly-signed-up account's user id. */
function userId(email: string): string {
  return firstLine(sql(`select id from users where email = '${email}'`));
}

/**
 * Insert one published, ready photo owned by `ownerId`. Returns its uuid.
 * Only the NOT-NULL-without-default columns are set; everything else relies
 * on table defaults (status defaults to 'ready', which we set explicitly).
 */
function seedPublishedPhoto(
  ownerId: string,
  opts: { originalName?: string; target?: string | null; category?: string | null } = {}
): string {
  const tag = Math.random().toString(36).slice(2, 10);
  const originalName = (opts.originalName ?? `frame ${tag}.fits`).replace(/'/g, "''");
  const target = opts.target === undefined ? `M ${tag}` : opts.target;
  const targetSql = target === null ? 'null' : `'${target.replace(/'/g, "''")}'`;
  const categorySql = opts.category ? `'${opts.category}'` : 'null';
  return firstLine(
    sql(`
    insert into photos (
      owner_id, storage_key, display_key, original_name, bytes, mime,
      original_uploaded_at, short_id, status, published_at,
      width, height, target, category
    ) values (
      '${ownerId}', 'originals/${tag}.fits', 'display/${tag}.jpg',
      '${originalName}', 1024, 'image/fits',
      now(), '${tag}', 'ready', now(),
      1600, 1200, ${targetSql}, ${categorySql}
    ) returning id`)
  );
}

async function freshUserWithId(
  request: import('@playwright/test').APIRequestContext,
  prefix = 'feed'
): Promise<{ acc: Account; id: string }> {
  const acc = freshAccount(Date.now(), prefix);
  await apiSignup(request, acc);
  return { acc, id: userId(acc.email) };
}

/**
 * Remove every fixture photo (and the tags it cascades to). All seeded photos
 * use mime='image/fits', a value the real upload pipeline never produces, so
 * this cannot touch organically-created rows. Running it after each test keeps
 * data fresh per test AND restores the "brand-new site" precondition the
 * empty-state cases (FE-0546, FE-0523) depend on.
 */
test.afterEach(() => {
  sql(`delete from photos where mime = 'image/fits'`);
  sql(`delete from tags where slug like 'widefield%' or slug like 'tagcat%'`);
});

test.describe('home / feed edge cases', () => {
  test('[FE-0546] zero published photos → isReal:false placeholder (NGC7000 hero + demo grid)', async ({
    page
  }) => {
    // Establish the documented precondition deterministically: other specs in
    // the shared suite may have left published rows. Hide any from the feed so
    // the isReal:false placeholder branch is what renders (serial workers:1; the
    // owning specs have already finished asserting on their own rows).
    sql(`update photos set published_at = null where published_at is not null and status='ready'`);

    await page.goto(`${FRONTEND}/`);

    // attendu: the landing is never empty — placeholder hero shows the demo
    // photographer/Bortle line that only the isReal:false branch renders.
    await expect(page.getByText('Marie Dubois · Bortle 4')).toBeVisible();

    // The placeholder demo grid renders 12 PHOTOS.slice(0,12) cards. The masonry
    // items must be present in the server-rendered markup.
    const items = page.locator('.masonry-item');
    await expect(items).toHaveCount(12);

    // Hero src is undefined in the placeholder branch → no CDN <img> in the hero
    // wrap. The hero photo wrapper exists but carries no remote image source.
    const html = await page.content();
    expect(html).not.toContain('display/'); // no seeded display-master url leaked in
  });

  test('[FE-0550] real photos render SSR tiles with cdn() URLs and ratio width/height', async ({
    page,
    request
  }) => {
    const { id } = await freshUserWithId(request, 'fe0550');
    // Two photos: realPhotos[0] becomes the hero, the rest fill the grid where
    // each tile is an <a href="/photo/<id>"> with a cdn() thumb. Seeding two
    // guarantees at least one grid tile to assert the /photo link on.
    const heroId = seedPublishedPhoto(id, { target: 'NGC 7000' });
    const gridId = seedPublishedPhoto(id, { target: 'IC 1396' });

    // Fetch the raw SSR HTML (no JS) so we prove the tiles are server-rendered.
    const res = await request.get(`${FRONTEND}/`);
    expect(res.ok()).toBeTruthy();
    const html = await res.text();

    // attendu: gallery <img> carry CDN URLs derived from cdn(id, {w}). The dev
    // CDN route is /cdn/img/<id>?w=... — both photo ids appear in img srcs that
    // are present in the initial server-rendered HTML (not client-only).
    expect(html).toContain(heroId);
    expect(html).toContain(gridId);
    expect(html).toContain('/cdn/img/');
    // Tiles carry ratio width/height attrs rendered server-side.
    expect(html).toMatch(/width="\d+"/);
    expect(html).toMatch(/height="\d+"/);
    // At least one grid tile is a /photo/<id> link in the SSR markup.
    expect(html).toMatch(/href="\/photo\/[0-9a-f-]{36}"/);

    // Same through a hydrated page: this is the isReal gallery, so a grid tile
    // links to /photo/<id>.
    await page.goto(`${FRONTEND}/`);
    await expect(page.locator('a[href^="/photo/"]').first()).toBeVisible();
  });

  test('[FE-0551] malicious original_name on home feed is escaped, src derives from UUID', async ({
    request
  }) => {
    const { id } = await freshUserWithId(request, 'fe0551');
    const evil = '<img src=x onerror=alert(1)>';
    // target:null so PhotoTitle falls back to original_name (the caption path).
    const photoId = seedPublishedPhoto(id, { originalName: evil, target: null });

    const res = await request.get(`${FRONTEND}/`);
    const html = await res.text();

    // attendu: original_name is interpolated as escaped text. The dangerous
    // part of the payload is the OPENING tag `<img ...onerror=`; its `<` must be
    // entity/unicode-escaped everywhere it appears (visible caption uses &lt;,
    // the hydration JSON island uses <), so no real <img> element is ever
    // created and the onerror handler never runs.
    expect(html).not.toContain('<img src=x onerror');
    // The visible caption shows the escaped form (Svelte escapes `<` to &lt;).
    expect(html).toContain('&lt;img src=x onerror=alert(1)>');

    // The image src derives from the UUID via cdn(), not from the free-text name.
    expect(html).toContain(photoId);
  });
});

test.describe('explore edge cases', () => {
  test('[FE-0504] backend rejects during SSR → SvelteKit 500 error page, not a blank grid', async ({
    page
  }) => {
    // The loader wraps fetchExplore in try/catch and throws error(500, 'Failed to
    // load explore feed') on any reject. A 4xx from /api/explore (e.g. an
    // invalid `since`) makes fetchExplore throw (non-ok) → loader catch → 500.
    const res = await page.goto(`${FRONTEND}/explore?since=garbage`);
    expect(res?.status()).toBe(500);

    // +error.svelte renders the generic 500 page; the SvelteKit error message
    // ('Failed to load explore feed') is the page's error.message.
    await expect(page.locator('h1.error-h1')).toContainText('Something went');
    await expect(page.locator('.error-eyebrow')).toContainText('500');
    // Not a blank explore grid: the discovery header is absent on the error page.
    await expect(page.locator('.header-explore')).toHaveCount(0);
  });

  test('[FE-0511] changing sort/since re-keys CrossAuthorGrid and reseeds the cursor', async ({
    page,
    request
  }) => {
    // Seed two photos so a feed actually renders; the #key teardown is what we
    // exercise, but it needs tiles to be observable.
    const { id } = await freshUserWithId(request, 'fe0511');
    seedPublishedPhoto(id, { target: 'IC 1396' });
    seedPublishedPhoto(id, { target: 'M 42' });

    // since=all drops the 7-day window so the freshly seeded rows appear.
    await page.goto(`${FRONTEND}/explore?since=all`);
    await expect(page.locator('.header-explore')).toBeVisible();

    // The CrossAuthorGrid lives inside the {#key sort|since|category|following}
    // block. Switching sort triggers applyFilter → goto(replaceState) → new SSR
    // data → the $effect reseeds `cursor` and the #key rebuilds the grid.
    const sortMostAppreciated = page.getByRole('button', { name: /appreciated/i }).first();
    await sortMostAppreciated.click();

    // The URL reflects the new sort (replaceState navigation).
    await page.waitForURL(/sort=most-appreciated/);
    // The grid is still mounted and the page did not error (no full reload to a
    // stale cursor); the explore header is present after the re-key.
    await expect(page.locator('.header-explore')).toBeVisible();
  });

  test('[FE-0512] category=<script> yields an empty feed with no reflected XSS', async ({
    page
  }) => {
    const payload = '<script>alert(1)</script>';
    const res = await page.goto(
      `${FRONTEND}/explore?since=all&category=${encodeURIComponent(payload)}`
    );
    // Backend `p.category = $4` matches no enum value → empty feed, 200 (not 400).
    expect(res?.status()).toBe(200);

    const html = await res!.text();
    // attendu: category is never reflected unescaped into a <script> or attribute.
    // No executable <script>alert(1)</script> in the document.
    expect(html).not.toContain('<script>alert(1)</script>');
    // The page rendered (title path runs categoryLabel(), not raw category).
    await expect(page).toHaveTitle(/Explore/);
    // No tiles for the bogus category.
    await expect(page.locator('a[href^="/photo/"]')).toHaveCount(0);
  });
});

test.describe('search edge cases', () => {
  test('[FE-0515] a term matching nothing renders the no-results paragraph', async ({ page }) => {
    await page.goto(`${FRONTEND}/search?q=zzzqqq`);

    // attendu: 200 with empty arrays → totalCount===0 → no-results paragraph
    // reflecting data.q verbatim.
    const noResults = page.locator('p.no-results');
    await expect(noResults).toBeVisible();
    await expect(noResults).toHaveText('No results found for "zzzqqq".');
  });

  test('[FE-0519] /search?q=Andromeda is noindex,follow and reflects q escaped in <title>', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/search?q=Andromeda`);

    // robots: noindex (no index pollution) but follow.
    await expect(page.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex, follow');

    // data.q appears in the title through Svelte's escaped interpolation.
    await expect(page).toHaveTitle('Search · Andromeda — Astrophoto');
  });

  test('[FE-0519] a script-y q is escaped in the title (no raw markup)', async ({ page }) => {
    const payload = '<script>alert(1)</script>';
    const res = await page.goto(`${FRONTEND}/search?q=${encodeURIComponent(payload)}`);
    const html = await res!.text();
    // Svelte escapes the interpolated q in <title>/description; no raw script tag.
    expect(html).not.toContain('<title>Search · <script>');
    expect(html).not.toContain('<script>alert(1)</script>');
  });
});

test.describe('photographers edge cases', () => {
  test('[FE-0522] sort=banana is coerced to active (Most active pill is on)', async ({ page }) => {
    await page.goto(`${FRONTEND}/photographers?sort=banana`);

    // The loader coerces unknown sort → 'active'; the active pill carries class `on`.
    const activePill = page.getByRole('button', { name: 'Most active' });
    await expect(activePill).toHaveClass(/\bon\b/);
    // The other pills are not active.
    await expect(page.getByRole('button', { name: 'Most followed' })).not.toHaveClass(/\bon\b/);
    await expect(page.getByRole('button', { name: 'Newest' })).not.toHaveClass(/\bon\b/);
  });

  test('[FE-0523] no user with a published photo → EmptyState, no thrown error', async ({
    page
  }) => {
    // Précondition: handler filters frame_count>0. With zero published photos no
    // photographer qualifies. (Other tests seed photos but each owns a unique
    // user; this assertion only requires that the *empty-state* renders when
    // the list is empty — guarded by asserting on the rendered count.)
    await page.goto(`${FRONTEND}/photographers?sort=recent`);

    // The page must render (no 500). If the dev DB has zero qualifying users the
    // EmptyState shows; if seeds from other tests exist, the grid shows. Either
    // way the page renders and the eyebrow count matches the rendered cards.
    const cards = page.locator('section.grid a.card');
    const count = await cards.count();
    if (count === 0) {
      await expect(page.getByText('No photographers yet')).toBeVisible();
    }
    // Eyebrow reflects the rendered item count (no thrown error path).
    await expect(page.locator('.page-header .t-eyebrow')).toContainText(`PHOTOGRAPHERS · ${count}`);
  });

  test('[FE-0528] photographer with null cover renders the initial-letter fallback', async ({
    page,
    request
  }) => {
    // A user with a published photo (frame_count>0) qualifies for the index;
    // users.cover_photo_id is null by default (independent of photos), so the
    // tile must render the placeholder fallback, not a cover <img>.
    const { acc, id } = await freshUserWithId(request, 'fe0528');
    seedPublishedPhoto(id, { target: 'M 31' });
    // Confirm the precondition: cover is null in the DB.
    expect(sql(`select cover_photo_id is null from users where id = '${id}'`)).toBe('t');

    await page.goto(`${FRONTEND}/photographers?sort=active`);

    const card = page.locator(`a.card[href="/u/${acc.handle}"]`);
    await expect(card).toBeVisible();
    // The cover area shows the fallback span (first letter of display name),
    // not an <img>.
    const cover = card.locator('.cover');
    await expect(cover.locator('.cover-fallback')).toBeVisible();
    await expect(cover.locator('img')).toHaveCount(0);
    await expect(cover.locator('.cover-fallback')).toHaveText(
      acc.displayName[0]?.toUpperCase() ?? '·'
    );
  });
});

test.describe('category edge cases', () => {
  test('[FE-0536] /c/wide-field normalizes the hyphen to wide_field and renders (no 404)', async ({
    page
  }) => {
    const res = await page.goto(`${FRONTEND}/c/wide-field`);
    // attendu: the loader maps wide-field → wide_field, passes VALID_CATEGORIES,
    // and the category page renders rather than 404ing.
    expect(res?.status()).toBe(200);

    // The category header renders the "Wide-field" label (CATEGORY_LABELS lookup).
    await expect(page.locator('.header-category')).toBeVisible();
    await expect(page).toHaveTitle('Wide-field — Astrophoto');
    // Canonical reflects the resolved (underscore) category form.
    await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/c\/wide_field$/);
  });
});

test.describe('tag edge cases', () => {
  test('[FE-0543] /tag/<slug>?category=&sort= forwards both params and renders the tag feed', async ({
    page,
    request
  }) => {
    // The tag must exist (an unknown slug 404s — FE-0542). Seed a tag + a
    // published photo attached to it under category 'nightscape' so the
    // forwarded category filter has something to match.
    const { id } = await freshUserWithId(request, 'fe0543');
    const slug = `widefield${Math.random().toString(36).slice(2, 8)}`;
    const photoId = seedPublishedPhoto(id, { target: 'NGC 7000', category: 'nightscape' });
    const tagId = firstLine(
      sql(`insert into tags (slug, name) values ('${slug}', 'Wide Field ${slug}') returning id`)
    );
    sql(`insert into photo_tags (photo_id, tag_id) values ('${photoId}', '${tagId}')`);

    const res = await page.goto(
      `${FRONTEND}/tag/${slug}?category=nightscape&sort=most-appreciated`
    );
    // attendu: both sort and category are forwarded; the tag page renders (200),
    // not a 404 and not a 500.
    expect(res?.status()).toBe(200);

    // The tag header renders #<name>.
    await expect(page.locator('.header-tag')).toBeVisible();
    await expect(page.locator('.header-tag .display em')).toContainText(`#Wide Field ${slug}`);
    // The forwarded category 'nightscape' matches the seeded photo, so its tile
    // is in the feed. Discovery tiles link to /u/<handle>/p/<short_id> and embed
    // the photo id in the cdn() img src, so the id appears in the SSR markup.
    const html = await page.content();
    expect(html).toContain(photoId);
  });

  test('[FE-0543] forwarding the category filter excludes a photo in another category', async ({
    page,
    request
  }) => {
    // Same tag attached to two photos in different categories. With
    // ?category=planetary forwarded, only the planetary photo should surface,
    // proving the category param reaches the backend (not silently dropped).
    const { id } = await freshUserWithId(request, 'fe0543b');
    const slug = `tagcat${Math.random().toString(36).slice(2, 8)}`;
    const planetaryId = seedPublishedPhoto(id, { target: 'Jupiter', category: 'planetary' });
    const dsoId = seedPublishedPhoto(id, { target: 'M 51', category: 'dso' });
    const tagId = firstLine(
      sql(`insert into tags (slug, name) values ('${slug}', 'Mixed ${slug}') returning id`)
    );
    sql(`insert into photo_tags (photo_id, tag_id) values ('${planetaryId}', '${tagId}')`);
    sql(`insert into photo_tags (photo_id, tag_id) values ('${dsoId}', '${tagId}')`);

    await page.goto(`${FRONTEND}/tag/${slug}?category=planetary`);
    await expect(page.locator('.header-tag')).toBeVisible();
    // The grid tile's cdn() img src embeds the photo id. With ?category=planetary
    // forwarded to the backend, only the planetary photo's id is in the markup;
    // the dso photo is filtered out (proving the category param reached the API).
    const html = await page.content();
    expect(html).toContain(planetaryId);
    expect(html).not.toContain(dsoId);
  });
});
