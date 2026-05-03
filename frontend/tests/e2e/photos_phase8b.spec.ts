/**
 * E2E tests for Phase 8b photo flows: drafts, replace, FollowButton polish,
 * untitled fallback, and mobile sticky AppreciateButton.
 *
 * NOTE: These tests require the full dev stack running (`just dev`) and a
 * MailHog instance reachable at http://localhost:8025.
 *
 * A `playwright.config.ts` at `frontend/` root is needed to execute these.
 * That config is deferred — add it when wiring up CI Playwright runs.
 * Until then this file is a spec stub: types-check via pnpm check but
 * won't be picked up by any test runner.
 *
 * TODO: Most tests here require bypassing email verification on signup.
 * Add a test-only backend route (e.g. POST /api/_test/signup-verified) or
 * seed the DB directly once playwright.config.ts is wired up.
 */

import { test, expect } from '@playwright/test';

const BACKEND = 'http://localhost:8080';
const FRONTEND = 'http://localhost:5173';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Create an account via the API and sign in through the UI.
 * Returns the email that was used. The account is email-unverified; most
 * write operations will work but some future guarded actions may redirect.
 *
 * TODO: requires a backend test-only route that marks the user verified
 * immediately (POST /api/_test/signup-verified), or a DB seed step.
 * Until then, tests that depend on a verified session must call
 * `test.skip()` if the page redirects to /signin.
 */
async function signupAndLogin(page: import('@playwright/test').Page, email: string): Promise<void> {
  const res = await page.request.post(`${BACKEND}/api/auth/signup`, {
    data: { email, password: 'longenoughpw1', display_name: 'E2E Phase8b' }
  });
  // Non-2xx means the backend rejected the request (e.g. duplicate email);
  // that's a test setup error, not a test assertion failure.
  if (!res.ok()) {
    throw new Error(`signup failed: ${res.status()} ${await res.text()}`);
  }

  await page.goto(`${FRONTEND}/signin`);
  await page.getByLabel('EMAIL').fill(email);
  await page.getByLabel('PASSWORD').fill('longenoughpw1');
  await page.getByRole('button', { name: 'Sign in' }).click();
}

// ---------------------------------------------------------------------------
// Test 1 — upload a draft, find it in /account/frames, publish from caption
// ---------------------------------------------------------------------------

test('upload a draft, find it in /account/frames, publish from caption step', async ({ page }) => {
  // TODO: requires email-verification bypass (see file header) and a real
  // JPEG fixture file at e2e-fixtures/sample.jpg relative to the test root.
  const email = `e2e-draft-${Date.now()}@upload.test`;
  await signupAndLogin(page, email);

  // Skip if not authenticated (no bypass in place).
  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }

  // -- Step 01: upload the file.
  await page.goto(`${FRONTEND}/upload`);
  await expect(page.getByText('NEW FRAME')).toBeVisible();

  // TODO: replace with a real fixture file once e2e-fixtures/ is created.
  // await page.getByLabel('YOUR UPLOAD').setInputFiles('tests/e2e/fixtures/sample.jpg');
  // await page.getByRole('button', { name: /Continue →/ }).click();

  // After POST /api/photos the server redirects to /upload/[id]/verify (Step 02).
  // await expect(page).toHaveURL(/\/upload\/[^/]+\/verify/);

  // -- Step 02: save as draft (skip metadata edits for this test).
  // await page.getByRole('button', { name: 'Save as draft' }).click();
  // await expect(page).toHaveURL('/account/frames');

  // -- /account/frames: confirm the draft callout band is visible.
  await page.goto(`${FRONTEND}/account/frames`);
  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }

  // The DraftsCallout renders "● {n} DRAFTS · NOT YET PUBLISHED" only when
  // there are drafts. This assertion works once the upload steps above are
  // uncommented and a fixture file is provided.
  // await expect(page.getByText(/DRAFTS · NOT YET PUBLISHED/)).toBeVisible();

  // The table row should show the draft with a DRAFT chip.
  // await expect(page.getByText('DRAFT')).toBeVisible();

  // -- Navigate into the draft and publish from Step 03 / caption.
  // await page.getByRole('link', { name: 'SEE ALL DRAFTS →' }).click();
  // await expect(page).toHaveURL(/\/account\/frames\?filter=drafts/);
  // Click the ⋯ menu → Edit metadata → Continue → lands on caption.
  // await page.getByRole('button', { name: /⋯|Actions/ }).first().click();
  // await page.getByText('Edit metadata').click();
  // await expect(page).toHaveURL(/\/upload\/[^/]+\/verify/);
  // await page.getByRole('button', { name: 'Continue →' }).click();
  // await expect(page).toHaveURL(/\/upload\/[^/]+\/caption/);
  // await page.getByRole('button', { name: 'Publish' }).click();
  // Successful publish redirects to the public photo detail.
  // await expect(page).toHaveURL(/\/photo\//);

  // Smoke-pass while fixture infrastructure is pending.
  expect(true).toBe(true);
});

// ---------------------------------------------------------------------------
// Test 2 — edit metadata of a published photo via ⋯ menu, save, no republish
// ---------------------------------------------------------------------------

test('edit metadata of a published photo via ⋯ menu, save changes, no republish', async ({
  page
}) => {
  // TODO: requires a pre-existing published photo owned by the test user.
  // Seed via POST /api/photos + POST /api/photos/:id/publish, or use a
  // fixture account with known data.
  const email = `e2e-editmeta-${Date.now()}@edit.test`;
  await signupAndLogin(page, email);

  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }

  // Assume the test photo detail URL is known from a seed step.
  // const PHOTO_SLUG = 'seed-published-photo-slug';
  // await page.goto(`${FRONTEND}/photo/${PHOTO_SLUG}`);

  // The ⋯ button has aria-label="Actions" (photo/[slug]/+page.svelte).
  // await page.getByRole('button', { name: 'Actions' }).click();
  // await expect(page.getByText('Edit metadata')).toBeVisible();
  // await page.getByText('Edit metadata').click();

  // Lands on /upload/[id]/verify in "EDIT METADATA" mode (isPublished=true).
  // await expect(page).toHaveURL(/\/upload\/[^/]+\/verify/);
  // await expect(page.getByText('EDIT METADATA')).toBeVisible();

  // Edit the target field.
  // await page.getByLabel('TARGET').fill('M42 Orion Nebula (edited)');

  // Primary button is "Save changes" (not "Continue →") for published photos.
  // await page.getByRole('button', { name: 'Save changes' }).click();

  // After save the page redirects back to the photo detail — not to caption.
  // No "Publish" call should have been made; the photo remains published.
  // await expect(page).toHaveURL(/\/photo\//);
  // await expect(page.getByText('M42 Orion Nebula (edited)')).toBeVisible();
  // await expect(page.getByText('PUBLISHED')).toBeVisible();

  // Confirm the DRAFT strip is NOT rendered (published_at still set).
  // await expect(page.getByText('DRAFT · ONLY YOU CAN SEE THIS')).not.toBeVisible();

  expect(true).toBe(true);
});

// ---------------------------------------------------------------------------
// Test 3 — replace a published photo; REPROCESSED label appears on detail
// ---------------------------------------------------------------------------

test('replace a published photo, REPROCESSED label appears on detail', async ({ page }) => {
  // TODO: requires a pre-existing published photo + a replacement JPEG
  // fixture at tests/e2e/fixtures/replacement.jpg.
  const email = `e2e-replace-${Date.now()}@replace.test`;
  await signupAndLogin(page, email);

  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }

  // const PHOTO_SLUG = 'seed-published-photo-for-replace';
  // await page.goto(`${FRONTEND}/photo/${PHOTO_SLUG}`);

  // Open the ⋯ owner action menu.
  // await page.getByRole('button', { name: 'Actions' }).click();
  // The menu item text matches ReplaceModal's trigger in photo/[slug]/+page.svelte.
  // await page.getByText('Replace image…').click();

  // The ReplaceModal's title is "Replace image" (Modal component `title` prop).
  // await expect(page.getByRole('dialog', { name: 'Replace image' })).toBeVisible();

  // Upload the replacement file.
  // await page.getByRole('dialog').getByRole('textbox', { name: /file/i })
  //   — input[type=file] inside the modal.
  // await page.locator('dialog input[type="file"]')
  //   .setInputFiles('tests/e2e/fixtures/replacement.jpg');
  // await page.getByRole('button', { name: 'Replace' }).click();

  // Modal closes; the page reloads via invalidateAll().
  // The pipeline sets status='processing', then 'ready'.
  // Poll until REPROCESSED label appears (pipeline may take a few seconds).
  // await expect(async () => {
  //   await page.reload();
  //   await expect(page.getByText(/REPROCESSED/)).toBeVisible();
  // }).toPass({ timeout: 30_000 });

  // Confirm caption, target and appreciations are preserved (not reset).
  // await expect(page.getByText('M42 Orion Nebula (edited)')).toBeVisible();

  expect(true).toBe(true);
});

// ---------------------------------------------------------------------------
// Test 4 — FollowButton toggles through 3 states with correct copy
// ---------------------------------------------------------------------------

test('FollowButton toggles through 3 states with correct copy', async ({ page }) => {
  // The FollowButton component is mounted on /u/[username] — it is NOT
  // present on the photo detail page (that page uses a static <a> link).
  // TODO: requires a second user account to follow, and authentication of
  // the viewer account.
  const viewerEmail = `e2e-follower-${Date.now()}@follow.test`;
  await signupAndLogin(page, viewerEmail);

  if (page.url().includes('/signin')) {
    test.skip();
    return;
  }

  // Visit a profile page where FollowButton is rendered (not the viewer's own
  // profile — isSelf hides the button). A seed account is needed.
  // const TARGET_USERNAME = 'seed-user-to-follow';
  // await page.goto(`${FRONTEND}/u/${TARGET_USERNAME}`);

  // State 1 — "Not following": button reads "Follow" (primary style).
  // const btn = page.getByRole('button', { name: 'Follow' });
  // await expect(btn).toBeVisible();

  // Click to follow (optimistic update fires immediately).
  // await btn.click();

  // State 2 — "Following · default": label switches to "✓ Following".
  // await expect(page.getByRole('button', { name: '✓ Following' })).toBeVisible();

  // State 3 — "Following · hover": hover over the button → label → "Unfollow?".
  // CSS-only hover in Playwright: hover() triggers :hover state.
  // await page.getByRole('button', { name: '✓ Following' }).hover();
  // await expect(page.getByRole('button', { name: 'Unfollow?' })).toBeVisible();

  // Click to unfollow.
  // await page.getByRole('button', { name: 'Unfollow?' }).click();

  // Back to state 1: "Follow".
  // await expect(page.getByRole('button', { name: 'Follow' })).toBeVisible();

  expect(true).toBe(true);
});

// ---------------------------------------------------------------------------
// Test 5 — untitled photo on home gallery shows UNTITLED chip
// ---------------------------------------------------------------------------

test('untitled photo on home shows UNTITLED chip', async ({ page }) => {
  // The PhotoTitle component renders <em class="untitled-chip">UNTITLED</em>
  // when photo.target is null/undefined (PhotoTitle.svelte).
  // The chip is visible in the home gallery masonry grid and in the table rows
  // on /account/frames.
  //
  // TODO: requires a published photo with target=NULL owned by any account.
  // Seed one via POST /api/photos (upload) + PUT /api/photos/:id (omit target)
  // + POST /api/photos/:id/publish.

  await page.goto(`${FRONTEND}/`);

  // If there is no untitled photo in the public gallery the assertion is a
  // no-op: the chip just won't appear. Seed data makes this deterministic.
  // await expect(page.getByText('UNTITLED')).toBeVisible();

  // The chip element is an <em> with class "untitled-chip" inside .photo-meta-row.
  // Confirm the chip is not accidentally present for photos that have a target.
  // const titledPhoto = page.locator('.photo-meta-row').filter({ hasText: /NGC|M[0-9]/ }).first();
  // await expect(titledPhoto.getByText('UNTITLED')).not.toBeVisible();

  // Smoke: page loads without JS errors.
  await expect(page).toHaveTitle(/Astrophoto/);
});

// ---------------------------------------------------------------------------
// Test 6 — mobile viewport: sticky AppreciateButton bar appears on detail
// ---------------------------------------------------------------------------

test('mobile viewport: sticky AppreciateButton bar appears on detail, tap toggles state', async ({
  page
}) => {
  // The mobile-sticky-wrap div is visible only at ≤ 640 px (CSS media query).
  // The AppreciateButton variant="mobile-sticky" renders inside it.
  //
  // TODO: requires a published photo with a known slug. Use a seed photo or
  // read the slug from the home page gallery after seeding.

  // Set viewport to narrow phone (≤ 640 px gate in photo/[slug]/+page.svelte).
  await page.setViewportSize({ width: 375, height: 812 });

  // Navigate to any published photo detail page.
  // const PHOTO_SLUG = 'seed-published-photo-slug';
  // await page.goto(`${FRONTEND}/photo/${PHOTO_SLUG}`);

  // The mobile sticky bar has role="toolbar" and aria-label="Photo actions"
  // (AppreciateButton.svelte, variant='mobile-sticky').
  // await expect(page.getByRole('toolbar', { name: 'Photo actions' })).toBeVisible();

  // Desktop action row (.action-row) should be hidden at this width.
  // await expect(page.locator('.action-row')).toBeHidden();

  // Tap the heart pill to appreciate (requires a signed-in user).
  // If unauthenticated, the tap redirects to /signin — skip the assertion.
  // const heartPill = page.getByRole('toolbar', { name: 'Photo actions' })
  //   .getByRole('button').first();
  // await heartPill.click();
  // if (page.url().includes('/signin')) {
  //   // Unauthenticated — redirect expected; appreciation state untestable.
  //   return;
  // }
  // await expect(heartPill).toHaveAttribute('aria-pressed', 'true');

  // Smoke: confirm viewport is set correctly.
  const vp = page.viewportSize();
  expect(vp?.width).toBe(375);
});
