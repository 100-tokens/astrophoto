/**
 * Profile-page surface audit regression suite (see the /u/<handle> prod
 * audit): the photographer page must be a complete document *without*
 * JavaScript — real <title>, real photo tiles, honest nav state — and
 * the feeds derived from it (RSS, og:image) must point at image URLs
 * that actually resolve.
 *
 * SSR is proven the house way: fetch raw HTML with the `request`
 * fixture (JS never runs), then re-check the hydrated page.
 */
import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup, sql } from './helpers';

// Marker mime for seeded rows — the real pipeline never produces it, so
// cleanup can bulk-delete safely (same convention as fe-feeds.spec.ts).
const MARKER_MIME = 'image/fits';

interface Seeded {
  handle: string;
  displayName: string;
  photoId: string;
  shortId: string;
}

/** Signup via API + publish one photo via SQL. 111h30m of integration. */
async function seedProfileWithPhoto(
  request: Parameters<typeof apiSignup>[0],
  ts: number
): Promise<Seeded> {
  const acc = freshAccount(ts, 'psurf');
  await apiSignup(request, acc);
  const userId = sql(`select id from users where email = '${acc.email}'`);
  const shortId = `PS${ts.toString(36)}`.slice(0, 12);
  // Pre-generate the id: psql -tAc appends the INSERT command tag to
  // `returning` output, so round-tripping the id through stdout breaks.
  const photoId = crypto.randomUUID();
  sql(
    `insert into photos
       (id, owner_id, storage_key, display_key, original_name, bytes, mime, status,
        short_id, target, width, height, integration_s,
        original_uploaded_at, published_at)
     values
       ('${photoId}', '${userId}', 'originals/psurf-${ts}.fits', 'display/psurf-${ts}.jpg',
        'final.xisf', 1000, '${MARKER_MIME}', 'ready',
        '${shortId}', 'NGC 5982', 2927, 2926, 401400, now(), now())`
  );
  // Give the profile an avatar so og:image has a source (cover/avatar
  // are its only inputs). Avatars share the display/<uuid>.jpg CDN
  // namespace, so any UUID makes a well-formed CDN URL.
  sql(`update users set avatar_id = '${photoId}' where id = '${userId}'`);
  return { handle: acc.handle, displayName: acc.displayName, photoId, shortId };
}

test.describe('profile page surface', () => {
  test.afterEach(() => {
    sql(`delete from photos where mime = '${MARKER_MIME}'`);
  });

  test('[PSURF-01] profile SSR: real title, tiles, integration stat, honest nav', async ({
    request
  }) => {
    const seeded = await seedProfileWithPhoto(request, Date.now());

    // Raw SSR HTML — no JS runs in the request fixture.
    const res = await request.get(`${FRONTEND}/u/${seeded.handle}`);
    expect(res.status()).toBe(200);
    const html = await res.text();

    // 1. The document title is the photographer, not the generic
    //    fallback (og:title was always right; <title> raced the layout).
    expect(html).toContain(`<title>${seeded.displayName} — Astrophoto</title>`);
    expect(html).not.toContain('<title>Astrophoto</title>');

    // 2. The photo grid is server-rendered: a real permalink anchor and
    //    a real <img> pointing at the CDN — previously the grid SSR'd as
    //    an empty <div style="height:0px"> because the justified layout
    //    only measured its container in onMount.
    expect(html).toContain(`/u/${seeded.handle}/p/${seeded.shortId}`);
    expect(html).toContain(`/cdn/img/${seeded.photoId}`);
    expect(html).not.toContain('style="height:0px"');

    // 3. Integration stat renders the XISF-decoded total (401400 s).
    expect(html).toContain('111h 30m');

    // 4. No nav link claims to be the current page on a profile route
    //    (the header used to default to Gallery/explore).
    expect(html).not.toContain('aria-current="page"');

    // 5. og:image exists and is a CDN URL (not the thumb API route,
    //    which 404s for XISF-style uploads without stored thumbnails).
    expect(html).toMatch(/property="og:image" content="[^"]*\/cdn\/img\//);
    expect(html).not.toContain('/thumb/1200');
  });

  test('[PSURF-02] hydrated profile matches SSR: tile visible and clickable', async ({
    page,
    request
  }) => {
    const seeded = await seedProfileWithPhoto(request, Date.now());

    await page.goto(`${FRONTEND}/u/${seeded.handle}`);
    const tile = page.locator(`a[href="/u/${seeded.handle}/p/${seeded.shortId}"]`);
    await expect(tile).toBeVisible();
    // Hydration must not blank the SSR'd grid (server/client identical).
    await expect(page.locator('.grid img')).toHaveCount(1);
    await expect(page).toHaveTitle(`${seeded.displayName} — Astrophoto`);
  });

  test('[PSURF-03] rss.xml enclosures use resolvable CDN URLs', async ({ request }) => {
    const seeded = await seedProfileWithPhoto(request, Date.now());

    const res = await request.get(`${FRONTEND}/rss.xml`);
    expect(res.status()).toBe(200);
    const xml = await res.text();

    // The seeded photo is the newest publish, so it must be in the feed
    // with a CDN image URL — never the /api/photos/<id>/thumb route.
    expect(xml).toContain(`/u/${seeded.handle}/p/${seeded.shortId}`);
    expect(xml).toMatch(new RegExp(`<enclosure url="[^"]*/cdn/img/${seeded.photoId}[^"]*"`));
    expect(xml).not.toContain('/api/photos/');

    // And the URL routes to the backend CDN handler, not a SvelteKit
    // 404 page. (Seeded rows have no object in MinIO, so the handler
    // may 404 — but as JSON from the image route, never as HTML. Full
    // 200-with-bytes coverage lives in the real-upload happy path.)
    const enclosure = xml.match(new RegExp(`<enclosure url="([^"]*${seeded.photoId}[^"]*)"`));
    const imgUrl = (enclosure?.[1] ?? '').replace(/&amp;/g, '&');
    expect(imgUrl).not.toBe('');
    const img = await request.get(imgUrl);
    expect(img.headers()['content-type'] ?? '').not.toContain('text/html');
  });
});
