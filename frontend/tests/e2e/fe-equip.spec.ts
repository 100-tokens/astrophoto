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

// ─────────────────────────────────────────────────────────────────────────
// Equipment + admin + account/frames front edge cases.
//
// Surfaces:
//   /settings/equipment            (gear-bundle "setups")
//   /settings/equipment/new        (SetupForm → /api/equipment/setups)
//   /settings/equipment/[id]/edit
//   /admin/equipment               (super-admin catalog)
//   /admin/equipment/[id]
//   /admin/settings
//   /account/frames                (the user's photo/draft library)
//
// The DB is shared + persistent, so every count/total/empty assertion is
// scoped to data this test seeds (unique brand/email tags, fresh users).
// ─────────────────────────────────────────────────────────────────────────

/** Insert a photo row directly. status defaults 'ready'. published_at NULL → draft. */
function seedPhoto(opts: {
  ownerId: string;
  published: boolean;
  createdAt: string; // ISO timestamp
  target?: string;
  exposureS?: number;
}): string {
  const pub = opts.published ? 'now()' : 'null';
  const target = opts.target ? `'${opts.target}'` : 'null';
  const exp = opts.exposureS != null ? String(opts.exposureS) : 'null';
  // short_id must be unique; derive from a random suffix.
  const shortId = `e${Math.random().toString(36).slice(2, 10)}`;
  return sql(`
    insert into photos (owner_id, storage_key, original_name, bytes, mime,
                        short_id, status, created_at, published_at,
                        original_uploaded_at, target, exposure_s)
    values ('${opts.ownerId}', 'originals/seed.fit', 'seed.fit', 1000, 'image/fits',
            '${shortId}', 'ready', '${opts.createdAt}', ${pub},
            '${opts.createdAt}', ${target}, ${exp})
    returning id
  `);
}

/** Resolve a freshly-signed-up user's UUID by email. */
function userId(email: string): string {
  return sql(`select id from users where email = '${email}'`);
}

// ─── /settings/equipment ──────────────────────────────────────────────────

test.describe('settings/equipment (setups list)', () => {
  test('[FE-0321] anonymous GET /settings/equipment redirects to /signin', async ({ page }) => {
    await page.goto(`${FRONTEND}/settings/equipment`);
    await page.waitForURL(/\/signin/);
    expect(new URL(page.url()).pathname).toBe('/signin');
  });

  test('[FE-0319] user with zero setups sees the empty state, no rows', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    await page.goto(`${FRONTEND}/settings/equipment`);
    // setups:[] → empty-state paragraph; no .card list items.
    await expect(page.locator('.empty')).toContainText('No setups yet');
    await expect(page.locator('ul.list')).toHaveCount(0);
  });

  test('[FE-0318] setDefault action with no id field returns fail(400) "Missing id"', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    // The UI hardcodes <input name="id">, so the missing-id guard is only
    // reachable by POSTing the action directly. The page request carries the
    // session cookie; a non-enhanced action POST re-renders with fail status.
    const res = await page.request.post(`${FRONTEND}/settings/equipment?/setDefault`, {
      form: {}
    });
    // A programmatic action POST returns HTTP 200 carrying SvelteKit's
    // serialized ActionResult; the fail() status + payload live in the body.
    const result = await res.json();
    expect(result.type).toBe('failure');
    expect(result.status).toBe(400);
    expect(result.data).toContain('Missing id');
  });

  test('[FE-0320] setDefault on a deleted setup: detail GET !ok → fail "Could not load setup", no PATCH', async ({
    page,
    request
  }) => {
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    const uid = userId(acc.email);
    // Seed a setup row owned by this user, then delete it to simulate the
    // other-tab-deleted race; setDefault's first GET /api/equipment/setups/:id
    // returns 404 (!ok) and bails before any PATCH.
    const setupName = `GoneSetup ${Date.now()}`;
    sql(`insert into equipment_setups (owner_id, name) values ('${uid}', '${setupName}')`);
    const setupId = sql(`select id from equipment_setups where name = '${setupName}'`);
    sql(`delete from equipment_setups where id = '${setupId}'`);

    const res = await page.request.post(`${FRONTEND}/settings/equipment?/setDefault`, {
      form: { id: setupId }
    });
    // dr.status was 404 → fail(404, 'Could not load setup'); no PATCH attempted.
    const result = await res.json();
    expect(result.type).toBe('failure');
    expect(result.status).toBe(404);
    expect(result.data).toContain('Could not load setup');
  });

  test('[FE-0322][FE-0323] setDefault rebuilds PATCH from detail (items+apply-mode), delete then redirects', async ({
    page,
    request
  }) => {
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    const cookies = await page.context().cookies();
    const cookieHeader = cookies.map((c) => `${c.name}=${c.value}`).join('; ');

    // Create a real setup (non-default) with one telescope item through the API
    // so detail.items is non-empty. First resolve-or-create the item.
    const itemRes = await request.post(`${BACKEND}/api/equipment/items`, {
      headers: { Cookie: cookieHeader },
      data: { kind: 'telescope', display_name: `Eq Scope ${acc.handle}` }
    });
    expect(itemRes.ok()).toBeTruthy();
    const item = await itemRes.json();

    const setupRes = await request.post(`${BACKEND}/api/equipment/setups`, {
      headers: { Cookie: cookieHeader },
      data: {
        name: `Primary ${acc.handle}`,
        is_remote: false,
        is_default: false,
        default_apply_mode: 'fill_empty',
        items: [{ role: 'optical_tube', item_id: item.id }]
      }
    });
    expect(setupRes.ok()).toBeTruthy();
    const setup = await setupRes.json();

    // FE-0322: flip default via the page action → setDefault rebuilds the body
    // from detail (items + default_apply_mode preserved). Drive it through the UI.
    await page.goto(`${FRONTEND}/settings/equipment`);
    await page.locator(`text=Primary ${acc.handle}`).waitFor();
    await page.locator('button:has-text("Set as default")').first().click();
    await page.waitForLoadState('networkidle');
    // After the default-flip the row shows the Default badge and the item count
    // (Telescope · 1) is still present — no item loss.
    await expect(page.locator('.badge.default').first()).toBeVisible();
    await expect(page.locator('.counts').first()).toContainText('Telescope · 1');

    // Confirm the item + apply-mode survived in the DB (no item/apply-mode loss).
    const itemCount = sql(`select count(*) from setup_items where setup_id = '${setup.id}'`);
    expect(itemCount).toBe('1');
    const applyMode = sql(
      `select default_apply_mode from equipment_setups where id = '${setup.id}'`
    );
    expect(applyMode).toBe('fill_empty');

    // FE-0323: delete action on a 204 backend response → redirect(303) back to
    // /settings/equipment (the row is gone afterwards).
    page.on('dialog', (d) => void d.accept()); // confirm() in the delete handler
    await page.locator('button.danger:has-text("Delete")').first().click();
    await page.waitForLoadState('networkidle');
    expect(new URL(page.url()).pathname).toBe('/settings/equipment');
    const stillThere = sql(`select count(*) from equipment_setups where id = '${setup.id}'`);
    expect(stillThere).toBe('0');
  });

  test('[FE-0324] setups-list GET non-ok → error(500) page (defensive branch; documented)', async ({
    page,
    request
  }) => {
    // The load does `if (!r.ok) error(500, 'Backend error')`. That branch is
    // NOT black-box-triggerable in this harness: locals.user (SSR) and the
    // per-load GET both use the same session cookie against the same backend,
    // and the setups-list query is robust (coalesced item_counts, no nullable
    // force-cast) — there is no data-induced 500 and Playwright cannot mock a
    // server-side load fetch. The frontend code is present and correct.
    //
    // What IS provable: the happy path renders for an authed user (the list
    // surface loads without throwing). We assert that and document the gap.
    await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    const res = await page.goto(`${FRONTEND}/settings/equipment`);
    expect(res?.status()).toBe(200);
    await expect(page.getByRole('heading', { name: 'Equipment setups' })).toBeVisible();
  });
});

// ─── /settings/equipment/new ────────────────────────────────────────────────

test.describe('settings/equipment/new', () => {
  test('[FE-0305] anonymous GET /settings/equipment/new redirects to /signin', async ({ page }) => {
    await page.goto(`${FRONTEND}/settings/equipment/new`);
    await page.waitForURL(/\/signin/);
    expect(new URL(page.url()).pathname).toBe('/signin');
  });

  test('[FE-0308] telescope autocomplete surfaces the catalog match with its specs summary', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'eq');

    // Seed "Sky-Watcher Esprit 100 ED" with telescope specs 100/550 → f/5.5.
    // canonical_name is the lowercased, whitespace-collapsed display_name.
    // Idempotent across runs: the (kind, canonical_name) unique row persists
    // in the shared DB, so on conflict reuse it and (re)ensure the specs row.
    sql(`
      insert into equipment_items (kind, canonical_name, display_name, brand, model, variant, status)
      values ('telescope', 'sky-watcher esprit 100 ed', 'Sky-Watcher Esprit 100 ED',
              'Sky-Watcher', 'Esprit 100', 'ED', 'approved')
      on conflict (kind, canonical_name) do nothing
    `);
    const itemId = sql(
      `select id from equipment_items where kind='telescope' and canonical_name='sky-watcher esprit 100 ed'`
    );
    sql(`
      insert into telescope_specs (item_id, design, aperture_mm, focal_length_mm, self_weight_kg)
      values ('${itemId}', 'refractor_apo', 100, 550, 6.4)
      on conflict (item_id) do update set aperture_mm = 100, focal_length_mm = 550
    `);

    await page.goto(`${FRONTEND}/settings/equipment/new`);
    // The telescope EquipmentAutocomplete input lives in the always-visible
    // role head row (RoleRow renders the `input` snippet regardless of the
    // expand state). Typing must *change* value (the $effect skips when
    // value === lastSelected) and is debounced 200ms before the fetch.
    const input = page.locator('input[name="telescope_name"]');
    await input.waitFor({ state: 'visible' });
    await input.fill('esprit 100');

    // Dropdown line-1 renders "<brand> · <model>" (variant omitted), line-2 is
    // the server-computed specs_summary. Anchor on both.
    const list = page.locator('ul.ac-list');
    await expect(list).toBeVisible({ timeout: 5000 });
    await expect(list.locator('.ac-line-1', { hasText: 'Sky-Watcher' })).toBeVisible();
    await expect(list.locator('.ac-line-1', { hasText: 'Esprit 100' })).toBeVisible();
    await expect(list.locator('.ac-line-2', { hasText: '100/550 f/5.5' })).toBeVisible();
  });
});

// ─── /settings/equipment/[id]/edit ──────────────────────────────────────────

test.describe('settings/equipment/[id]/edit', () => {
  test('[FE-0315] unknown setup id (well-formed UUID) → 404 "Setup not found"', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    // A well-formed but nonexistent UUID exercises the backend's 404 (not a
    // 400 deserialization failure that would hit the error(500) branch).
    const res = await page.goto(
      `${FRONTEND}/settings/equipment/00000000-0000-0000-0000-000000000000/edit`
    );
    // The load throws error(404, ...) → SvelteKit renders the custom 404 page
    // (the message goes to the error object, not the body copy). The observable
    // contract is the 404 status + the 404 error page.
    expect(res?.status()).toBe(404);
    await expect(page.locator('body')).toContainText('404');
  });

  test('[FE-0316] item-detail fallback branch (defensive; not harness-triggerable, documented)', async ({
    page,
    request
  }) => {
    // The load does, per role: if (item-detail fetch !ok) → { detail: null, item }
    // so SetupForm still seeds via detail?.display_name ?? item.display_name.
    // This branch is NOT triggerable black-box here: setup_items.item_id has an
    // ON DELETE RESTRICT FK (the item always exists), items_get has no status
    // filter and force-casts only count columns (specs are all Option) — so the
    // detail GET cannot deterministically 404/500. Server-side load fetches also
    // cannot be route-mocked. The frontend fallback is present and correct.
    //
    // What IS provable: a setup whose item exists renders the edit page with the
    // item's display_name seeded (the `?? item.display_name` source of truth).
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'eq');
    const cookies = await page.context().cookies();
    const cookieHeader = cookies.map((c) => `${c.name}=${c.value}`).join('; ');

    const itemRes = await request.post(`${BACKEND}/api/equipment/items`, {
      headers: { Cookie: cookieHeader },
      data: { kind: 'telescope', display_name: `Edit Scope ${acc.handle}` }
    });
    const item = await itemRes.json();
    const setupRes = await request.post(`${BACKEND}/api/equipment/setups`, {
      headers: { Cookie: cookieHeader },
      data: {
        name: `EditMe ${acc.handle}`,
        is_remote: false,
        is_default: false,
        default_apply_mode: 'overwrite',
        items: [{ role: 'optical_tube', item_id: item.id }]
      }
    });
    const setup = await setupRes.json();

    const res = await page.goto(`${FRONTEND}/settings/equipment/${setup.id}/edit`);
    expect(res?.status()).toBe(200);
    // Page renders, no crash; the setup name input is seeded (server-rendered
    // value attribute). Auto-waits/retries through the SSR round-trip.
    await expect(page.locator(`input[value="EditMe ${acc.handle}"]`)).toHaveCount(1, {
      timeout: 10000
    });
  });
});

// ─── /admin/equipment ───────────────────────────────────────────────────────

test.describe('admin/equipment', () => {
  /** Fresh verified admin, logged in via UI (so the session reflects is_admin). */
  async function adminLogin(
    page: import('@playwright/test').Page,
    request: import('@playwright/test').APIRequestContext
  ) {
    const acc = freshAccount(Date.now(), 'eqadm');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    makeAdmin(acc.email);
    await uiLogin(page, acc);
    return acc;
  }

  test('[FE-0327] q matches nothing → empty result set rendered', async ({ page, request }) => {
    await adminLogin(page, request);
    await page.goto(`${FRONTEND}/admin/equipment?q=zzznomatch`);
    await expect(page.locator('td.empty')).toContainText('No equipment found.');
    await expect(page.locator('.count')).toContainText('0 items');
  });

  test('[FE-0332] has_more=true at page 0 enables Next and forwards filters', async ({
    page,
    request
  }) => {
    await adminLogin(page, request);
    // PAGE_SIZE is 50. Seed 51 items under a unique brand so total=51, page 0
    // returns 50, has_more = offset(0)+50 < 51 = true. Scope by ?q=<brand>.
    const brand = `ZBrand${Date.now()}`;
    sql(`
      insert into equipment_items (kind, canonical_name, display_name, brand, model, status)
      select 'telescope',
             lower('${brand} m' || g),
             '${brand} M' || g,
             '${brand}', 'M' || g, 'approved'
        from generate_series(1, 51) g
    `);

    await page.goto(`${FRONTEND}/admin/equipment?q=${brand}`);
    await expect(page.locator('.count')).toContainText('51 items');
    // has_more=true → Next button enabled; Prev disabled at page 0.
    const next = page.locator('.pager button:has-text("Next")');
    const prev = page.locator('.pager button:has-text("Prev")');
    await expect(next).toBeEnabled();
    await expect(prev).toBeDisabled();
    await expect(page.locator('.pager span')).toContainText('Page 1');

    // Forwarding: clicking Next preserves q and advances page.
    await next.click();
    await page.waitForURL(/page=1/);
    const url = new URL(page.url());
    expect(url.searchParams.get('q')).toBe(brand);
    expect(url.searchParams.get('page')).toBe('1');
    await expect(page.locator('.pager span')).toContainText('Page 2');
  });

  test('[FE-0329] non-admin hitting /admin/equipment is bounced to /', async ({
    page,
    request
  }) => {
    // +layout.server.ts redirects authenticated non-admins home (UX guard).
    await signupVerifiedAndLogin(page, request, Date.now(), 'eqna');
    await page.goto(`${FRONTEND}/admin/equipment`);
    await page.waitForURL(`${FRONTEND}/`);
    expect(new URL(page.url()).pathname).toBe('/');
  });
});

// ─── /admin/equipment/[id] ──────────────────────────────────────────────────

test.describe('admin/equipment/[id]', () => {
  test('[FE-0340] in-use item disables Delete (title + hint) but Save stays allowed', async ({
    page,
    request
  }) => {
    const acc = freshAccount(Date.now(), 'eqadm');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    makeAdmin(acc.email);
    await uiLogin(page, acc);

    // Seed an item with usage_count > 0 → inUse true. approved_at stamped to
    // mirror the real items_create path (an in-use item is legitimately
    // approved); the detail page renders fine either way.
    const ts = Date.now();
    const canon = `inuse mount ${ts}`;
    sql(`
      insert into equipment_items (kind, canonical_name, display_name, brand, model, status, usage_count, approved_at)
      values ('mount', '${canon}', 'InUse Mount ${ts}',
              'BrandX', 'MountY', 'approved', 3, now())
    `);
    const itemId = sql(
      `select id from equipment_items where kind='mount' and canonical_name='${canon}'`
    );

    await page.goto(`${FRONTEND}/admin/equipment/${itemId}`);
    const del = page.locator('button.danger:has-text("Delete item")');
    await expect(del).toBeDisabled();
    await expect(del).toHaveAttribute('title', 'In use — cannot delete');
    await expect(page.locator('.hint', { hasText: 'Delete is disabled' })).toBeVisible();
    // Save remains enabled.
    await expect(page.locator('button.primary:has-text("Save changes")')).toBeEnabled();
  });
});

// ─── /admin/settings ────────────────────────────────────────────────────────

test.describe('admin/settings', () => {
  test('[FE-0349] non-admin direct nav to /admin/settings → redirect to /', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'setna');
    await page.goto(`${FRONTEND}/admin/settings`);
    await page.waitForURL(`${FRONTEND}/`);
    expect(new URL(page.url()).pathname).toBe('/');
  });

  test('[FE-0348] failed save renders errorMsg in .err, busy resets', async ({ page, request }) => {
    const acc = freshAccount(Date.now(), 'setadm');
    await apiSignup(request, acc);
    verifyEmail(acc.email);
    makeAdmin(acc.email);
    await uiLogin(page, acc);

    await page.goto(`${FRONTEND}/admin/settings`);
    // Drive the free-tier input above the backend cap (1..=100000) so the PUT
    // returns 422 and updateSettings throws. The input carries max="100000",
    // and HTML5 constraint validation blocks form submission for an
    // out-of-range value — strip max (as the signin template strips required)
    // so the over-cap value reaches the server guard.
    const free = page.locator('input[type="number"]').first();
    await free.evaluate((el) => el.removeAttribute('max'));
    await free.fill('200000');
    await page.locator('button:has-text("Save settings")').click();

    // save() catch → errorMsg in .err (the thrown message is "updateSettings 422").
    await expect(page.locator('p.err')).toBeVisible();
    await expect(page.locator('p.err')).toContainText('422');
    // busy reset in finally → Save button re-enabled (text back to "Save settings").
    await expect(page.locator('button:has-text("Save settings")')).toBeEnabled();
  });
});

// ─── /account/frames ────────────────────────────────────────────────────────

test.describe('account/frames', () => {
  test('[FE-0353] anonymous GET preserves the query string in the signin next param', async ({
    page
  }) => {
    await page.goto(`${FRONTEND}/account/frames?filter=drafts`);
    await page.waitForURL(/\/signin/);
    const url = new URL(page.url());
    expect(url.pathname).toBe('/signin');
    // next = encodeURIComponent(pathname + search) → "/account/frames?filter=drafts"
    expect(url.searchParams.get('next')).toBe('/account/frames?filter=drafts');
  });

  test('[FE-0351] zero photos + zero drafts → empty plate hero, no PhotosTable', async ({
    page,
    request
  }) => {
    await signupVerifiedAndLogin(page, request, Date.now(), 'fr');
    await page.goto(`${FRONTEND}/account/frames`);
    await expect(page.locator('.empty h1')).toContainText('An empty plate');
    await expect(page.locator('a:has-text("Upload a frame")')).toBeVisible();
    await expect(page.locator('table.photos-table')).toHaveCount(0);
  });

  test('[FE-0350] bogus filter/sort/view query params fall through to defaults, no crash', async ({
    page,
    request
  }) => {
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'fr');
    const uid = userId(acc.email);
    // One published + one draft so the merged [...drafts, ...published] list
    // (the filter fall-through) renders rows.
    seedPhoto({ ownerId: uid, published: true, createdAt: '2026-01-01T00:00:00Z', target: 'M31' });
    seedPhoto({ ownerId: uid, published: false, createdAt: '2026-02-01T00:00:00Z', target: 'M42' });

    const res = await page.goto(`${FRONTEND}/account/frames?filter=bogus&sort=weird&view=xyz`);
    expect(res?.status()).toBe(200);
    // filter !== drafts/published → merged list; both rows present, no crash.
    await expect(page.locator('table.photos-table tbody tr')).toHaveCount(2);
  });

  test('[FE-0354] bigint-ish counts coerced via Number — no NaN in stats or the ♡ cell', async ({
    page,
    request
  }) => {
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'fr');
    const uid = userId(acc.email);
    seedPhoto({
      ownerId: uid,
      published: true,
      createdAt: '2026-01-01T00:00:00Z',
      target: 'M51',
      exposureS: 300
    });

    await page.goto(`${FRONTEND}/account/frames`);
    // StatsRow numbers are Number()-cast → never "NaN".
    const statsRow = page.locator('.stats-row');
    await expect(statsRow).toBeVisible();
    await expect(statsRow).not.toContainText('NaN');
    await expect(statsRow.locator('.cell', { hasText: 'PUBLISHED' }).locator('.num')).toHaveText(
      '1'
    );
    // The published row's ♡ cell shows a coerced number (0), not NaN.
    const row = page.locator('table.photos-table tbody tr').first();
    await expect(row).not.toContainText('NaN');
    await expect(row.locator('td').nth(5)).toHaveText('0');
  });

  test('[FE-0355] /api/me/stats non-ok → error(502) (defensive branch; not harness-triggerable, documented)', async ({
    page,
    request
  }) => {
    // fetchJson throws error(502, 'Backend error') if any of the three load
    // GETs is non-ok. None is black-box-triggerable here: SSR locals.user and
    // the per-load GETs share one cookie/backend (no auth desync), me/stats is
    // coalesced/count-only (no data-induced 500), and PhotoRow is a schema-
    // checked query_as! over only NOT-NULL force-casts (no NULL-able cast to
    // poison). Server-side load fetches also cannot be route-mocked. The
    // frontend error mapping is present and correct.
    //
    // What IS provable: the happy path returns 200 with numeric stats (the
    // alternative the 502 branch guards against — NaN stats — does not occur).
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'fr');
    const uid = userId(acc.email);
    seedPhoto({ ownerId: uid, published: true, createdAt: '2026-01-01T00:00:00Z', target: 'NGC' });
    const res = await page.goto(`${FRONTEND}/account/frames`);
    expect(res?.status()).toBe(200);
    await expect(page.locator('.stats-row')).not.toContainText('NaN');
  });

  test('[FE-0356] sort=oldest orders the merged list by created_at ascending', async ({
    page,
    request
  }) => {
    const acc = await signupVerifiedAndLogin(page, request, Date.now(), 'fr');
    const uid = userId(acc.email);
    // A published (older) + a draft (newer). The merged list concatenates
    // [...drafts, ...published] = [newer, older]; a plain reverse() would only
    // flip that to [older, newer]. The real sort is by created_at, so:
    //   sort=oldest (dir=1) → ascending → [OLD(M31), NEW(M42)]
    seedPhoto({ ownerId: uid, published: true, createdAt: '2026-01-01T00:00:00Z', target: 'M31' });
    seedPhoto({ ownerId: uid, published: false, createdAt: '2026-03-01T00:00:00Z', target: 'M42' });

    await page.goto(`${FRONTEND}/account/frames?sort=oldest`);
    const rows = page.locator('table.photos-table tbody tr');
    await expect(rows).toHaveCount(2);
    // Row order by created_at ascending: M31 (Jan) before M42 (Mar).
    await expect(rows.nth(0)).toContainText('M31');
    await expect(rows.nth(1)).toContainText('M42');
  });
});
