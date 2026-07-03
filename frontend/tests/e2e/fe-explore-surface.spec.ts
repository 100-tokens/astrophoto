/**
 * Explore surface audit regression suite. Highlights:
 *  - [XSURF-01] hydration integrity: the tile used to nest the author
 *    <a> inside the tile <a>; the parser split the DOM and SvelteKit
 *    hydration crashed (HierarchyRequestError) on every discovery
 *    route, silently re-rendering client-side.
 *  - SSR tiles are real markup, anonymous ?following=true no longer
 *    dead-ends, and category variants canonicalize to /c/<cat>.
 */
import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup, sql } from './helpers';

const MARKER_MIME = 'image/x-xsurf-check';

function seedExplorePhoto(userId: string, target: string): string {
  const photoId = crypto.randomUUID();
  sql(
    `insert into photos
       (id, owner_id, storage_key, display_key, original_name, bytes, mime, status,
        short_id, target, width, height, original_uploaded_at, published_at)
     values
       ('${photoId}', '${userId}', 'originals/${photoId}', 'display/${photoId}.jpg',
        'xsurf.jpg', 1000, '${MARKER_MIME}', 'ready',
        '${photoId.slice(0, 8)}', '${target}', 1600, 1200, now(), now())`
  );
  return photoId;
}

test.describe('explore surface', () => {
  test.afterEach(() => {
    sql(`delete from photos where mime = '${MARKER_MIME}'`);
  });

  test('[XSURF-01] /explore hydrates without errors and tiles are valid multi-link cards', async ({
    page,
    request
  }) => {
    const acc = freshAccount(Date.now(), 'xsurf');
    await apiSignup(request, acc);
    const userId = sql(`select id from users where email = '${acc.email}'`);
    seedExplorePhoto(userId, 'NGC 891');

    const warnings: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'warning' || msg.type() === 'error') warnings.push(msg.text());
    });

    await page.goto(`${FRONTEND}/explore?since=all`);
    await page.waitForLoadState('networkidle');

    expect(
      warnings.filter((w) => w.includes('hydrate') || w.includes('Hydration')),
      warnings.join('\n')
    ).toHaveLength(0);

    // The parsed DOM keeps the caption inside the tile (the nested-<a>
    // bug hoisted caption fragments out as free-floating grid items).
    const tile = page.locator('.grid article.tile').first();
    await expect(tile).toBeVisible();
    await expect(tile.locator('a.tile-link')).toHaveCount(1);
    await expect(tile.locator('.cap .author-chip')).toHaveCount(1);
  });

  test('[XSURF-02] SSR HTML carries real tile anchors and no nested links', async ({ request }) => {
    const acc = freshAccount(Date.now(), 'xsurf');
    await apiSignup(request, acc);
    const userId = sql(`select id from users where email = '${acc.email}'`);
    const photoId = seedExplorePhoto(userId, 'NGC 6946');

    const res = await request.get(`${FRONTEND}/explore?since=all`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    expect(html).toContain(`/u/${acc.handle}/p/`);
    expect(html).toContain(`/cdn/img/${photoId}`);
    // No <a> opened inside another <a> anywhere in the grid.
    const grid = html.slice(html.indexOf('class="grid'), html.indexOf('</main>'));
    for (const m of grid.matchAll(/<a [^>]*>((?:(?!<\/a>).)*?)<a /gs)) {
      expect(m[0], 'nested anchor in grid').toBe('');
    }
  });

  test('[XSURF-03] anonymous ?following=true shows the full feed, not a dead end', async ({
    request
  }) => {
    const acc = freshAccount(Date.now(), 'xsurf');
    await apiSignup(request, acc);
    const userId = sql(`select id from users where email = '${acc.email}'`);
    seedExplorePhoto(userId, 'IC 5070');

    const res = await request.get(`${FRONTEND}/explore?since=all&following=true`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    // The loader strips `following` for anonymous sessions — the seeded
    // photo renders instead of the misleading "be the first to publish"
    // empty state (whose responsible pill is hidden for anon).
    expect(html).toContain(`/u/${acc.handle}/p/`);
    expect(html).not.toContain('No frames here yet');
  });

  test('[XSURF-04] category variants canonicalize to /c/<cat>; plain explore to itself', async ({
    request
  }) => {
    const plain = await (await request.get(`${FRONTEND}/explore`)).text();
    expect(plain).toContain(`<link rel="canonical" href="${FRONTEND}/explore"`);

    const dso = await (await request.get(`${FRONTEND}/explore?category=dso`)).text();
    expect(dso).toContain(`<link rel="canonical" href="${FRONTEND}/c/dso"`);
  });
});
