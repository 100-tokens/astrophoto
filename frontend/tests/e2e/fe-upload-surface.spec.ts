/**
 * Upload-flow surface audit regression suite (see the /upload prod
 * audit). Highlights:
 *  - [USURF-03] the critical one: XISF-style async metadata back-fill
 *    must survive Publish (the one-shot form seed used to send
 *    key-present nulls that wiped everything the header recovered).
 *  - anonymous SSR redirects, humanized wizard errors, the 'pending'
 *    dead-end band, and drafts pagination with the RFC3339 cursor.
 */
import { test, expect } from '@playwright/test';
import { FRONTEND, freshAccount, apiSignup, verifyEmail, uiLogin, sql } from './helpers';

const MARKER_MIME = 'image/x-usurf-check';

function seedDraft(
  userId: string,
  opts: { status: string; displayKey?: string | null; createdAgoMin?: number } = { status: 'ready' }
): string {
  const photoId = crypto.randomUUID();
  const display =
    opts.displayKey === null ? 'null' : `'${opts.displayKey ?? `display/${photoId}.jpg`}'`;
  const ago = opts.createdAgoMin ?? 0;
  sql(
    `insert into photos
       (id, owner_id, storage_key, display_key, original_name, bytes, mime, status,
        last_step, short_id, original_uploaded_at, created_at)
     values
       ('${photoId}', '${userId}', 'originals/${photoId}', ${display},
        'usurf.xisf', 1000, '${MARKER_MIME}', '${opts.status}',
        'upload', '${photoId.slice(0, 8)}', now(), now() - interval '${ago} minutes')`
  );
  return photoId;
}

test.describe('upload flow surface', () => {
  test.afterEach(() => {
    sql(`delete from photos where mime = '${MARKER_MIME}'`);
  });

  test('[USURF-01] anonymous /upload and verify deep-links redirect to signin', async ({
    request
  }) => {
    for (const path of ['/upload', `/upload/${crypto.randomUUID()}/verify`]) {
      const res = await request.get(`${FRONTEND}${path}`, { maxRedirects: 0 });
      expect(res.status(), path).toBe(303);
      expect(res.headers()['location'], path).toContain('/signin');
    }
  });

  test('[USURF-03] async metadata back-fill survives Publish', async ({ page, request }) => {
    const acc = freshAccount(Date.now(), 'usurf');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    await uiLogin(page, acc);
    // uiLogin clicks submit without awaiting the redirect — racing
    // straight into page.goto can beat the Set-Cookie.
    await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });
    const userId = sql(`select id from users where email = '${acc.email}'`);

    // An XISF mid-calibration: the verify form seeds from this nearly
    // empty row and is disabled while the page polls.
    const photoId = seedDraft(userId, { status: 'awaiting-calibration' });

    await page.goto(`${FRONTEND}/upload/${photoId}/verify`);
    await expect(page.getByText('PLATE-SOLVING XISF').first()).toBeVisible();

    // The background pipeline back-fills header metadata, then readies.
    sql(
      `update photos
          set camera = 'ZWO ASI533MM PRO', exposure_s = 300, gain = 100,
              sessions = 24, target = 'NGC 5982', status = 'ready'
        where id = '${photoId}'`
    );

    // The 2s poll flips the page to the enabled form, and the back-fill
    // must be ADOPTED into the (empty) form fields.
    await expect(page.locator('input[name="camera"]')).toHaveValue('ZWO ASI533MM PRO', {
      timeout: 15000
    });
    await expect(page.locator('input[name="exposure_s"]')).toHaveValue('300');

    // Publish sends the full form snapshot — before the adoption fix,
    // these fields went up as key-present nulls and were CLEARED.
    await page.getByRole('button', { name: 'Publish', exact: true }).click();
    await page.waitForURL(/\/(photo|u)\//, { timeout: 15000 });

    const camera = sql(`select camera from photos where id = '${photoId}'`);
    const exposure = sql(`select exposure_s from photos where id = '${photoId}'`);
    const target = sql(`select target from photos where id = '${photoId}'`);
    expect(camera).toBe('ZWO ASI533MM PRO');
    expect(exposure).toBe('300');
    expect(target).toBe('NGC 5982');
  });

  test('[USURF-04] a pending draft dead-ends honestly instead of an enabled form', async ({
    page,
    request
  }) => {
    const acc = freshAccount(Date.now(), 'usurf');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    await uiLogin(page, acc);
    // uiLogin clicks submit without awaiting the redirect — racing
    // straight into page.goto can beat the Set-Cookie.
    await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });
    const userId = sql(`select id from users where email = '${acc.email}'`);
    const photoId = seedDraft(userId, { status: 'pending', displayKey: null });

    await page.goto(`${FRONTEND}/upload/${photoId}/verify`);
    await expect(page.getByText('UPLOAD INCOMPLETE', { exact: false }).first()).toBeVisible();
    await expect(page.getByRole('button', { name: 'Publish', exact: true })).toHaveCount(0);

    // Discard actually deletes the row now (it used to wipe metadata
    // via an empty save_draft and leave the draft in place). Wait for
    // hydration — the click handler is a Svelte action, not a form.
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: 'Discard' }).click();
    await page.waitForURL(/\/account\/frames/, { timeout: 15000 });
    const left = sql(`select count(*) from photos where id = '${photoId}'`);
    expect(left).toBe('0');
  });

  test('[USURF-05] drafts pagination survives the RFC3339 cursor', async ({ page, request }) => {
    const acc = freshAccount(Date.now(), 'usurf');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    await uiLogin(page, acc);
    // uiLogin clicks submit without awaiting the redirect — racing
    // straight into page.goto can beat the Set-Cookie.
    await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });
    const userId = sql(`select id from users where email = '${acc.email}'`);

    // A full page (24) + 1 so next_cursor is emitted. Distinct
    // created_at so the strict less-than cursor doesn't skip rows.
    for (let i = 0; i < 25; i++) {
      seedDraft(userId, { status: 'ready', createdAgoMin: i + 1 });
    }

    await page.goto(`${FRONTEND}/me/drafts`);
    const older = page.getByRole('link', { name: 'Older →' });
    await expect(older).toBeVisible();
    await older.click();

    // The '+00:00' in the cursor used to decode to a space → backend
    // 400 'bad cursor' → error page. Page 2 must render the remainder
    // (the tile's visible title is 'untitled'; the filename is img alt).
    await page.waitForURL(/cursor=/, { timeout: 15000 });
    await expect(page.getByRole('img', { name: 'usurf.xisf' }).first()).toBeVisible();
    await expect(page.getByText('Internal Error')).toHaveCount(0);
  });
});
