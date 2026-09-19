import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup, verifyEmail, uiLogin, sql } from './helpers';

/**
 * Regression: clicking a photo in the profile gallery opens the full detail
 * view (Edit / Replace / Delete) at the permalink.
 */

function firstLine(out: string): string {
  return (out.split('\n')[0] ?? '').trim();
}

test('[owner] clicking own photo in the profile gallery opens the full view with Delete', async ({
  page,
  request
}) => {
  const acc = freshAccount(Date.now(), 'del');
  await apiSignup(request, acc);
  verifyEmail(acc.email);
  await uiLogin(page, acc);

  const ownerId = firstLine(sql(`select id from users where email = '${acc.email}'`));
  const shortId = `del${Date.now().toString(36)}`.slice(0, 18);
  sql(
    `insert into photos
       (owner_id, storage_key, display_key, original_name, bytes, mime,
        original_uploaded_at, short_id, status, published_at, width, height, target)
     values
       ('${ownerId}', 'originals/${shortId}.jpg', 'display/${shortId}.jpg', 'x.jpg', 1024,
        'image/jpeg', now(), '${shortId}', 'ready', now(), 1600, 1200, 'M51 Whirlpool')`
  );

  try {
    await page.goto(`${FRONTEND}/u/${acc.handle}`);

    // The tile is an <a> with the canonical permalink. Wait for hydration, then click.
    const tile = page.locator(`a[href="/u/${acc.handle}/p/${shortId}"]`).first();
    await expect(tile).toBeVisible();
    await page.waitForLoadState('networkidle');

    await tile.click();

    // Owner → full detail view at the permalink, with the Delete affordance.
    await expect(page).toHaveURL(new RegExp(`/u/${acc.handle}/p/${shortId}`), { timeout: 15000 });
    await expect(page.locator('.action-delete')).toBeVisible({ timeout: 15000 });
  } finally {
    sql(`delete from photos where short_id = '${shortId}'`);
  }
});
