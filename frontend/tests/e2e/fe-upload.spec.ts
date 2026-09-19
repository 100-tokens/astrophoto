import { test, expect } from '@playwright/test';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { FRONTEND, BACKEND, sql, signupVerifiedAndLogin } from './helpers';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SAMPLE_JPEG = readFileSync(join(__dirname, 'fixtures', 'sample.jpg'));

/**
 * Resolve a user's UUID from their email. Seeding photos needs the owner_id.
 */
function userId(email: string): string {
  return sql(`select id from users where email = '${email}'`);
}

/**
 * Seed a photo row directly. GET /api/photos/:id reads the DB and needs no
 * S3 object, so a bare row is enough to reach every server-rendered state
 * (processing / ready / draft / published). Returns the new photo id.
 *
 * status defaults to 'ready'. published_at null ⇒ is_draft true (a draft).
 * display_key is set for ready rows so the verify/draft views don't assume a
 * missing master beyond what each case intends.
 */
function seedPhoto(
  ownerEmail: string,
  opts: {
    status?: string;
    published?: boolean;
    originalName?: string;
    displayKey?: string | null;
    exifJson?: string | null;
    createdAt?: string;
  } = {}
): string {
  const id = randomUUID();
  const owner = userId(ownerEmail);
  const status = opts.status ?? 'ready';
  const name = opts.originalName ?? `seed-${id.slice(0, 8)}.jpg`;
  const shortId = id.replace(/-/g, '').slice(0, 11);
  const publishedAt = opts.published ? 'now()' : 'null';
  const displayKey =
    opts.displayKey === undefined
      ? `'display/${id}.jpg'`
      : opts.displayKey === null
        ? 'null'
        : `'${opts.displayKey}'`;
  const exifJson = opts.exifJson ? `'${opts.exifJson}'::jsonb` : 'null';
  const createdAt = opts.createdAt ? `'${opts.createdAt}'` : 'now()';

  sql(
    `insert into photos
       (id, owner_id, storage_key, original_name, bytes, mime, status,
        original_uploaded_at, short_id, display_key, published_at, last_step,
        exif_json, created_at)
     values
       ('${id}', '${owner}', 'originals/${id}.jpg', '${name}', 38198,
        'image/jpeg', '${status}', now(), '${shortId}', ${displayKey},
        ${publishedAt}, 'verify', ${exifJson}, ${createdAt})`
  );
  return id;
}

// ─────────────────────────────────────────────────────────────────────────
// /upload
// ─────────────────────────────────────────────────────────────────────────

test('[FE-0400] dropping a 60 MiB file on a free-tier user opens TierUpgradeModal and fires no upload-init', async ({
  page,
  request
}) => {
  await signupVerifiedAndLogin(page, request, Date.now(), 'up');
  await page.goto(`${FRONTEND}/upload`);
  await page.waitForLoadState('networkidle');

  // Precondition: this account is on the free tier (50 MB / file). Pin it so a
  // tier-default change can't make the size gate pass for the wrong reason.
  await expect(page.getByText('FREE TIER · 50 MB / FILE')).toBeVisible();

  // Spy: the size gate (onFiles: file.size > TIER_MAX) runs *before* preflight
  // or any fetch, so no /api/uploads/init must ever be issued for this file.
  let initFired = false;
  page.on('request', (r) => {
    if (r.method() === 'POST' && r.url().includes('/api/uploads/init')) initFired = true;
  });

  // 60 MiB > 50 MiB free cap. The file never reaches preflight/decode (the
  // gate skips it), so a non-decodable file of zeros is fine. Playwright caps
  // inline buffers at 50 MB, so write it to disk and pass the path.
  const bigPath = join(tmpdir(), `fe-0400-${randomUUID()}.jpg`);
  writeFileSync(bigPath, Buffer.alloc(60 * 1024 * 1024, 0));
  await page.locator('input[type="file"]').setInputFiles(bigPath);

  // attendu: TierUpgradeModal opens.
  await expect(page.getByRole('dialog', { name: 'Upgrade tier' })).toBeVisible();
  // attendu: no slot pushed — the file list (rendered only when slots.length)
  // never appears.
  await expect(page.locator('.file-list')).toHaveCount(0);
  // attendu: no POST /api/uploads/init fired for that file.
  await page.waitForTimeout(500);
  expect(initFired).toBe(false);
});

test('[FE-0413] cancelling an upload past 50% opens a confirm dialog ("complete will be lost") before DELETE fires', async ({
  page,
  request
}) => {
  await signupVerifiedAndLogin(page, request, Date.now(), 'up');

  // Throttle the upload so real bytes stream slowly and progress climbs past
  // 50% while still in the 'uploading' state. Route interception would make
  // xhr.upload.onprogress jump 0→100, so we throttle the network instead.
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: 0,
    downloadThroughput: -1,
    uploadThroughput: 700_000 // bytes/s — ~12 MB body ⇒ ~17s upload window
  });

  // Build a large but valid JPEG: sample.jpg + trailing zero padding. JPEG
  // decoders ignore bytes after the EOI marker, so createImageBitmap (in
  // preflight) still succeeds; the full padded body is what gets PUT to S3.
  const padded = Buffer.concat([SAMPLE_JPEG, Buffer.alloc(12 * 1024 * 1024, 0)]);

  let deleteFired = false;
  page.on('request', (r) => {
    if (r.method() === 'DELETE' && /\/api\/uploads\/[0-9a-f-]+$/.test(r.url())) deleteFired = true;
  });

  await page.goto(`${FRONTEND}/upload`);
  await page.waitForLoadState('networkidle');

  await page.locator('input[type="file"]').setInputFiles({
    name: 'big-but-valid.jpg',
    mimeType: 'image/jpeg',
    buffer: padded
  });

  // Wait until the slot is uploading past 50% (the progressbar exposes
  // aria-valuenow). cancelSlot only opens the dialog when pct > 50.
  const bar = page.getByRole('progressbar', { name: 'Upload progress' });
  await expect(bar).toBeVisible({ timeout: 30_000 });
  await expect
    .poll(async () => Number((await bar.getAttribute('aria-valuenow')) ?? '0'), {
      timeout: 30_000,
      intervals: [200]
    })
    .toBeGreaterThan(50);

  // The DELETE must not have fired yet — the confirm gate comes first.
  expect(deleteFired).toBe(false);

  // Click the per-row cancel button (× icon, aria-label "Cancel upload of …").
  await page.getByRole('button', { name: /Cancel upload of/ }).click();

  // attendu: ConfirmDialog opens with "X% complete will be lost" BEFORE the
  // DELETE /api/uploads/:id fires.
  const dialog = page.getByRole('dialog', { name: 'Cancel upload' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('complete will be lost');
  expect(deleteFired).toBe(false);

  // Confirming then issues the DELETE.
  await dialog.getByRole('button', { name: 'Cancel upload' }).click();
  await expect.poll(() => deleteFired, { timeout: 5_000 }).toBe(true);
});

test('[FE-0415] a published photo carrying embedded GPS EXIF never surfaces exif_json on the public API', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');

  // Seed a PUBLISHED photo whose exif_json carries embedded GPS coordinates —
  // exactly what kamadak-exif's blanket `reader.fields()` capture would persist
  // from a GPS-tagged JPEG (see backend/src/photos/exif.rs::parse_blocking).
  const gps = JSON.stringify({
    GPSLatitude: '48 deg 51 min 29.6 sec N',
    GPSLongitude: '2 deg 17 min 40.2 sec E',
    Model: 'Canon EOS Ra'
  });
  const id = seedPhoto(acc.email, { published: true, exifJson: gps });

  // attendu (the assertable exposure side): fetch the public photo detail as
  // an ANONYMOUS client. PhotoDetail (get.rs) projects a fixed field whitelist
  // and never serializes exif_json, so the GPS site must not leak. ra_deg/
  // dec_deg are celestial coords (solve/manual), not the terrestrial site, so
  // they are exposed by design and intentionally not asserted here.
  const res = await request.get(`${BACKEND}/api/photos/${id}`);
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  expect(body).not.toHaveProperty('exif_json');
  const raw = await res.text();
  expect(raw).not.toContain('GPSLatitude');
  expect(raw).not.toContain('GPSLongitude');
  expect(raw).not.toContain('48 deg 51 min');
});

// ─────────────────────────────────────────────────────────────────────────
// /upload/[id]/verify
// ─────────────────────────────────────────────────────────────────────────

test('[FE-0419] a still-processing JPEG shows the processing state and keeps Publish gated until ready', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');
  const id = seedPhoto(acc.email, { status: 'processing', displayKey: null });

  await page.goto(`${FRONTEND}/upload/${id}/verify`);
  await page.waitForLoadState('networkidle');

  // attendu: VerifyPane shows the processing state…
  await expect(page.locator('p.processing-meta')).toContainText('PROCESSING THUMBNAILS');
  // …and the publish action stays gated (disabled) while status === processing.
  const publish = page.getByRole('button', { name: 'Publish' });
  await expect(publish).toBeVisible();
  await expect(publish).toBeDisabled();
});

test('[FE-0421] queued autosave PUTs abort the prior in-flight request so only the latest survives', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');
  // Must be a READY draft: a processing photo disables the fieldset (can't
  // type) and a published photo skips autosave entirely.
  const id = seedPhoto(acc.email, { status: 'ready', published: false });

  // Hold every PUT /api/photos/:id ~4s so the first save is still in flight
  // when the second debounced save fires and aborts it. The browser abort
  // rejects the held route mid-wait.
  await page.route('**/api/photos/*', async (route) => {
    if (route.request().method() !== 'PUT') return route.continue();
    await new Promise((r) => setTimeout(r, 4000));
    try {
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    } catch {
      // route already aborted by the browser — nothing to fulfill.
    }
  });

  const puts: { url: string; status: 'finished' | 'failed' }[] = [];
  page.on('requestfinished', (r) => {
    if (r.method() === 'PUT' && r.url().includes(`/api/photos/${id}`))
      puts.push({ url: r.url(), status: 'finished' });
  });
  page.on('requestfailed', (r) => {
    if (r.method() === 'PUT' && r.url().includes(`/api/photos/${id}`))
      puts.push({ url: r.url(), status: 'failed' });
  });

  await page.goto(`${FRONTEND}/upload/${id}/verify`);
  await page.waitForLoadState('networkidle');

  const caption = page.locator('#verify-caption');
  // Edit 1 → wait past the 1500ms debounce so PUT #1 is issued and held.
  await caption.fill('first edit value');
  await page.waitForTimeout(1800);
  // Edit 2 → wait past the debounce so PUT #2 fires; its launch aborts PUT #1.
  await caption.fill('second edit value WINS');
  await page.waitForTimeout(1800);

  // Let PUT #2 complete (route holds ~4s).
  await expect.poll(() => puts.length, { timeout: 10_000 }).toBeGreaterThanOrEqual(2);

  // attendu: the superseded (earlier) PUT is AbortController-cancelled; only
  // the latest survives.
  const failed = puts.filter((p) => p.status === 'failed');
  const finished = puts.filter((p) => p.status === 'finished');
  expect(failed.length).toBeGreaterThanOrEqual(1); // PUT #1 aborted
  expect(finished.length).toBeGreaterThanOrEqual(1); // PUT #2 survived
});

test('[FE-0428] a normal photo renders the idle plate-solve panel without erroring the page', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');
  const id = seedPhoto(acc.email, { status: 'ready', published: false });

  // The platesolve-status fetch runs server-side in load() and CANNOT be
  // forced to throw from Playwright (page.route only sees browser requests).
  // Its catch leaves platesolveStatus = null — the SAME end-state as a normal
  // unsolved photo. So we assert the resilience end-state: the page renders
  // 200 with the plate-solve panel in its idle state, not an error page.
  const resp = await page.goto(`${FRONTEND}/upload/${id}/verify`);
  expect(resp?.status()).toBe(200);
  await page.waitForLoadState('networkidle');

  // The verify form rendered (not an error boundary) and the plate-solve block
  // is present in its non-solving idle state.
  await expect(page.locator('form.metadata-form')).toBeVisible();
  await expect(page.getByText('PLATE SOLVE', { exact: false }).first()).toBeVisible();
});

// ─────────────────────────────────────────────────────────────────────────
// /me/drafts
// ─────────────────────────────────────────────────────────────────────────

test('[FE-0449] a still-processing draft (no display master) renders a thumb pointing at /img/<id>?w=320', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');
  const id = seedPhoto(acc.email, { status: 'processing', published: false, displayKey: null });

  await page.goto(`${FRONTEND}/me/drafts`);
  await page.waitForLoadState('networkidle');

  // attendu: DraftTile renders thumb_url = {cdn}/img/<id>?w=320, which has no
  // display/<id>.jpg yet ⇒ placeholder/broken thumb until the pipeline writes
  // the master. The observable is the src pattern + the processing pip.
  const img = page.locator(`.tile .thumb img`).first();
  await expect(img).toHaveAttribute('src', new RegExp(`/img/${id}\\?w=320`));
  await expect(page.locator('.status[data-state="processing"]')).toContainText('processing');
});

test('[FE-0453] a user with zero drafts sees the EmptyState and no Resume-recent button', async ({
  page,
  request
}) => {
  // Fresh account ⇒ zero drafts.
  await signupVerifiedAndLogin(page, request, Date.now(), 'up');

  await page.goto(`${FRONTEND}/me/drafts`);
  await page.waitForLoadState('networkidle');

  // attendu: EmptyState ("No drafts yet", cta → /upload); Resume recent hidden.
  await expect(page.getByText('No drafts yet')).toBeVisible();
  const cta = page.getByRole('link', { name: /Upload a frame/i });
  await expect(cta).toHaveAttribute('href', '/upload');
  await expect(page.getByRole('button', { name: 'Resume recent' })).toHaveCount(0);
});

test('[FE-0454] Resume recent filters to drafts inside the newest-60min window, all owner-scoped', async ({
  page,
  request
}) => {
  const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'up');

  // Two recent drafts (within 60 min of the newest) + one old draft (2h back).
  // resumeRecent should carry only the two recent ids into verify.
  const recentA = seedPhoto(acc.email, { status: 'ready', published: false });
  const recentB = seedPhoto(acc.email, {
    status: 'ready',
    published: false,
    createdAt: new Date(Date.now() - 10 * 60 * 1000).toISOString()
  });
  const old = seedPhoto(acc.email, {
    status: 'ready',
    published: false,
    createdAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString()
  });

  await page.goto(`${FRONTEND}/me/drafts`);
  await page.waitForLoadState('networkidle');

  await page.getByRole('button', { name: 'Resume recent' }).click();
  await page.waitForURL(/\/upload\/[^/]+\/verify\?ids=/);

  // attendu: only the two within-window ids go into the one-by-one verify
  // queue; the old one is excluded. The list itself is owner-filtered.
  const url = new URL(page.url());
  const ids = (url.searchParams.get('ids') ?? '').split(',').filter(Boolean);
  expect(ids).toContain(recentA);
  expect(ids).toContain(recentB);
  expect(ids).not.toContain(old);
  expect(ids.length).toBe(2);
});
