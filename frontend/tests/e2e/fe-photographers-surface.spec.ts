/**
 * Photographers surface audit regression suite: card stats are truthful
 * through the whole stack (the 33e2575 fanout fix finally gets an
 * end-to-end pin), and the a11y affordances added by the audit exist.
 */
import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup, sql } from './helpers';

const MARKER_MIME = 'image/x-psurf-check';

function seedReadyPhoto(userId: string, opts: { integrationS?: number } = {}): string {
  const photoId = crypto.randomUUID();
  const integration = opts.integrationS != null ? String(opts.integrationS) : 'null';
  sql(
    `insert into photos
       (id, owner_id, storage_key, display_key, original_name, bytes, mime, status,
        short_id, width, height, integration_s, original_uploaded_at, published_at)
     values
       ('${photoId}', '${userId}', 'originals/${photoId}', 'display/${photoId}.jpg',
        'psurf.jpg', 1000, '${MARKER_MIME}', 'ready',
        '${photoId.slice(0, 8)}', 1600, 1200, ${integration}, now(), now())`
  );
  return photoId;
}

test.describe('photographers surface', () => {
  test.afterEach(() => {
    sql(`delete from photos where mime = '${MARKER_MIME}'`);
  });

  test('[PSRF-01] card stats are truthful with followers (fanout regression, end to end)', async ({
    page,
    request
  }) => {
    const star = freshAccount(Date.now(), 'psurf');
    await apiSignup(request, star);
    const starId = sql(`select id from users where email = '${star.email}'`);
    seedReadyPhoto(starId, { integrationS: 7200 });
    seedReadyPhoto(starId);
    // Two followers — the pre-33e2575 join multiplied frames and
    // integration by follower count.
    for (let i = 0; i < 2; i++) {
      const fan = freshAccount(Date.now() + i + 1, 'psurffan');
      await apiSignup(request, fan);
      const fanId = sql(`select id from users where email = '${fan.email}'`);
      sql(`insert into follows (follower_id, followed_id) values ('${fanId}', '${starId}')`);
    }

    await page.goto(`${FRONTEND}/photographers?sort=followers`);
    const card = page.locator(`a.card[href="/u/${star.handle}"]`);
    await expect(card).toBeVisible();
    const stats = card.locator('.stats');
    // 2 frames (not 4), 2h integration (not 4h), 2 followers. The
    // spans have no literal whitespace between number and label (CSS
    // gap), so match with \s*.
    await expect(stats).toContainText(/2\s*frames/);
    await expect(stats).toContainText(/2h\s*integration/);
    await expect(stats).toContainText(/2\s*followers/);
    // The accessible name distinguishes same-display-name photographers.
    await expect(card).toHaveAttribute('aria-label', `${star.displayName} (@${star.handle})`);
  });

  test('[PSRF-02] sort pills expose pressed state; SSR carries the cards', async ({ request }) => {
    const acc = freshAccount(Date.now(), 'psurf');
    await apiSignup(request, acc);
    const userId = sql(`select id from users where email = '${acc.email}'`);
    seedReadyPhoto(userId);

    const res = await request.get(`${FRONTEND}/photographers?sort=recent`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    expect(html).toContain(`/u/${acc.handle}`);
    // The active pill carries aria-pressed="true", the others "false".
    expect(html).toMatch(/aria-pressed="true"[^>]*>Newest|Newest<[^]*aria-pressed="true"/);
    expect((html.match(/aria-pressed="true"/g) ?? []).length).toBe(1);
    expect((html.match(/aria-pressed="false"/g) ?? []).length).toBe(2);
  });

  test('[PSRF-03] bad sort coerces; the API contract rejects it strictly', async ({ request }) => {
    // The page coerces (never 500s)...
    const pageRes = await request.get(`${FRONTEND}/photographers?sort=banana`);
    expect(pageRes.status()).toBe(200);
    // ...while the API is strict for programmatic consumers.
    const backend = process.env.PLAYWRIGHT_BACKEND_URL ?? 'http://localhost:8080';
    const apiRes = await request.get(`${backend}/api/photographers?sort=banana`);
    expect(apiRes.status()).toBe(400);
    const cursorRes = await request.get(`${backend}/api/photographers?cursor=garbage`);
    expect(cursorRes.status()).toBe(400);
  });
});
