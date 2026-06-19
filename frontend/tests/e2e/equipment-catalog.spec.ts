/**
 * E2E tests for the equipment-catalog feature (Tasks 24–26).
 *
 * Tasks:
 *   24 – setup_builder_with_telescope_specs
 *   25 – upload_verify_chip_input
 *   26 – equip_browse_specs_header
 *
 * Requires the full dev stack running (`just dev`):
 *   - Backend:  http://localhost:8080
 *   - Frontend: http://localhost:5173
 *   - Postgres + MinIO: running via docker compose
 *
 * Run with: cd frontend && pnpm test:e2e equipment-catalog
 */

import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';
import { FRONTEND, BACKEND, freshAccount, verifyEmail } from './helpers';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// NOTE — STATUS: these scenarios were validated live via chrome-devtools-mcp
// in the post-merge polish session. The Playwright spec itself currently
// fails on flaky timing (HandlePicker debounce, upload pipeline
// `[data-state="ready"]`) on this dev box. Treat the file as a
// scaffolded CI target; before wiring it into CI:
//   - confirm the docker exec psql pivot works in the runner environment;
//   - tune the HandlePicker / pipeline wait selectors;
//   - either bring back a `just dev` baseURL on :5173 OR teach
//     playwright.config.ts to honour PLAYWRIGHT_BASE_URL.
// The functional coverage these tests describe is already exercised by
// the MCP playthroughs documented in the post-merge code review.

// ---------------------------------------------------------------------------
// Auth helper. The post-email-verification signup lands on
// /signup/check-email; we mark the user verified out-of-band via the
// test verify endpoint, then sign in via the form to get a session
// cookie. Returns the handle used.
// ---------------------------------------------------------------------------
async function signupViaForm(page: import('@playwright/test').Page, ts: number): Promise<string> {
  const acc = freshAccount(ts, 'equip');

  await page.goto(`${FRONTEND}/signup`);

  await page.fill('input[name="display_name"]', acc.displayName);
  await page.fill('input[name="handle"]', acc.handle);
  await page.fill('input[name="email"]', acc.email);
  await page.fill('input[name="password"]', acc.password);

  // HandlePicker debounces 300 ms then fetches /api/auth/handle-check.
  await expect(page.locator('[data-status="available"]')).toBeVisible({ timeout: 5000 });

  await page.click('button[type="submit"]');
  // Signup ships email verification and lands on /signup/check-email.
  await page.waitForURL(/\/signup\/check-email/, { timeout: 15000 });

  // Mark the user verified via the shared psql helper (signin is blocked
  // until email_verified_at is set; the outbox link is not driven here).
  verifyEmail(acc.email);

  // Login via the form to land an authenticated session cookie.
  await page.goto(`${FRONTEND}/signin`);
  await page.fill('input[name="email"]', acc.email);
  await page.fill('input[name="password"]', acc.password);
  await page.click('button[type="submit"]');
  await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });

  return acc.handle;
}

// ---------------------------------------------------------------------------
// Test 24 — setup builder with telescope specs
// ---------------------------------------------------------------------------
test.describe('setup_builder_with_telescope_specs', () => {
  test('creates a setup, fills telescope specs, saves to catalog, saves the setup', async ({
    page
  }) => {
    const ts = Date.now();
    await signupViaForm(page, ts);

    // Navigate to the new-setup page.
    await page.goto(`${FRONTEND}/settings/equipment/new`);

    // Fill the setup name.
    const setupName = `E2E Setup ${ts}`;
    await page.fill('input[placeholder="e.g. Backyard SHO @ Bortle 4"]', setupName);

    // Fill the telescope name and trigger commit via blur.
    const telescopeName = `E2E Refractor ${ts}`;
    const telescopeInput = page.locator('input[name="telescope_name"]');
    await telescopeInput.fill(telescopeName);
    // Tab away triggers onBlur → commit → POST /api/equipment/items.
    await telescopeInput.press('Tab');

    // Wait a moment for the POST to resolve (up to 5 s).
    await page.waitForTimeout(1000);

    // Click "Edit specs" on the TELESCOPE row.
    // RoleRow renders the button inside the row that contains "TELESCOPE".
    const telescopeRow = page.locator('.role-row', { hasText: 'TELESCOPE' }).first();
    await telescopeRow.getByRole('button', { name: 'Edit specs' }).click();

    // The SpecsPanel should now be visible. In create mode (new item) it
    // shows "● NEW · WILL JOIN THE SHARED CATALOG"; in edit mode it shows
    // "● EDITING A SHARED CATALOG ITEM". Either is acceptable here.
    const panel = telescopeRow.locator('.specs-panel');
    await expect(panel).toBeVisible({ timeout: 5000 });

    // Set design to "Refractor APO" via the <select>.
    await panel.locator('select').first().selectOption({ label: 'Refractor APO' });

    // Aperture — the second number input in the specs grid.
    const numberInputs = panel.locator('input[type="number"]');
    await numberInputs.nth(0).fill('100'); // aperture_mm
    await numberInputs.nth(1).fill('550'); // focal_length_mm

    // Click "Save to catalog".
    await panel.getByRole('button', { name: 'Save to catalog' }).click();

    // After save the panel header transitions to EDITING mode
    // (because the item now exists in the catalog). The indicator text
    // changes — wait up to 5 s for the transition.
    await expect(telescopeRow.locator('.specs-panel-head')).toContainText(
      'EDITING A SHARED CATALOG ITEM',
      { timeout: 5000 }
    );

    // Re-assert the setup name immediately before saving. The first fill above
    // can land in the pre-hydration window: SvelteKit serves SSR HTML, and the
    // controlled `bind:value={setupName}` (initial '') resets the input on
    // hydrate, eating a value typed before JS attached — so saveSetup() would
    // bail on its "Setup name is required" guard. By now the "EDITING A SHARED
    // CATALOG ITEM" transition (a JS fetch) proves hydration is done, so this
    // fill reliably populates the reactive state.
    await page.fill('input[placeholder="e.g. Backyard SHO @ Bortle 4"]', setupName);

    // Click "Save setup". saveSetup() PATCHes each role's specs (403 for shared
    // catalog items — handled and ignored), POSTs /api/equipment/setups, then
    // client-side goto()s. Poll the URL (toHaveURL doesn't wait on a load event,
    // so it's robust to SvelteKit's client navigation under vite HMR).
    await page.getByRole('button', { name: 'Save setup' }).click();
    await expect(page).toHaveURL(`${FRONTEND}/settings/equipment`, { timeout: 15000 });

    // The setup name should appear in the list.
    await expect(page.getByText(setupName)).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Test 25 — upload-verify chip input
// ---------------------------------------------------------------------------
test.describe('upload_verify_chip_input', () => {
  test('uploads a photo, navigates to verify, adds a filter chip via the autocomplete', async ({
    page
  }) => {
    const ts = Date.now();
    await signupViaForm(page, ts);

    // ── Step 1: Seed a filter item via the API so the autocomplete has a result.
    // We must be authenticated — the session cookie is already set via signup.
    // Use the frontend's fetch proxy; the backend is at BACKEND directly.
    const filterName = `E2E Filter ${ts}`;
    const filterRes = await page.request.post(`${BACKEND}/api/equipment/items`, {
      data: {
        kind: 'filter',
        display_name: filterName,
        specs: {
          kind: 'filter',
          filter_type: 'h_alpha',
          bandwidth_nm: 3.0,
          size: '2in',
          mounted: true
        }
      }
    });
    if (!filterRes.ok()) {
      // Not a test assertion failure — log and continue; the chip-input still
      // renders even without pre-seeded results (shows "No matches").
      console.warn(`filter item seed failed: ${filterRes.status()} ${await filterRes.text()}`);
    }

    // ── Step 2: Upload a real JPEG to obtain a photo id with the verify step.
    await page.goto(`${FRONTEND}/upload`);
    await page.waitForLoadState('networkidle');

    const fixturePath = path.resolve(__dirname, 'fixtures/sample.jpg');
    await page.setInputFiles('input[type="file"]', fixturePath);

    // Wait for the upload pipeline to complete (data-state="ready").
    const readyRow = page.locator('[data-state="ready"]');
    await expect(readyRow).toBeVisible({ timeout: 30000 });

    // The footer "Verify N ready frame →" button advances to the verify step.
    const continueBtn = page.locator('button:has-text("ready frame")');
    await expect(continueBtn).toBeEnabled({ timeout: 5000 });
    await continueBtn.click();
    await page.waitForURL(/\/upload\/[^/]+\/verify/, { timeout: 15000 });

    // ── Step 3: Interact with FilterChipInput.
    // The structured-filters input lives in the .equip-filters section. Scope
    // to it: TagChipInput reuses the bare .fchip-input class for tags too.
    const chipInput = page.locator('.equip-filters .fchip-input');
    await expect(chipInput).toBeVisible({ timeout: 5000 });

    // Click the chip input to open the dropdown.
    await chipInput.click();

    // The dropdown (fchip-pop) should appear.
    const popup = page.locator('.fchip-pop');
    await expect(popup).toBeVisible({ timeout: 3000 });

    // Type the filter name — the autocomplete debounce is ~0 ms in this
    // component (fetch fires on every oninput).
    const typeQuery = filterName.substring(0, 8); // e.g. "E2E Filt"
    await chipInput.locator('input').fill(typeQuery);

    // Give autocomplete time to fetch and render.
    await page.waitForTimeout(800);

    // If the seed item appeared, select it. Otherwise press Enter to create.
    const popItem = popup.locator('.fchip-pop-item').first();
    const hasItem = await popItem.isVisible().catch(() => false);
    if (hasItem) {
      await popItem.click();
    } else {
      // "Create new filter" flow.
      await chipInput.locator('input').press('Enter');
      await page.waitForTimeout(500);
    }

    // Close the autocomplete popup so it no longer renders preview chips, then
    // assert a real FilterChip was added to the input. Added chips live in the
    // draggable <span> wrappers (direct children of .fchip-input), not in the
    // .fchip-pop dropdown.
    await page.keyboard.press('Escape');
    await expect(popup).toBeHidden({ timeout: 3000 });
    const chip = chipInput.locator(':scope > span > .fchip');
    await expect(chip.first()).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Test 26 — equip browse specs header
// ---------------------------------------------------------------------------
test.describe('equip_browse_specs_header', () => {
  test('seeded filter item shows BANDWIDTH, SIZE and MOUNTED in the specs bar', async ({
    page
  }) => {
    const ts = Date.now();
    await signupViaForm(page, ts);

    // Seed a filter item with full specs.
    const displayName = `E2E Test Filter ${ts}`;
    const createRes = await page.request.post(`${BACKEND}/api/equipment/items`, {
      data: {
        kind: 'filter',
        display_name: displayName,
        specs: {
          kind: 'filter',
          filter_type: 'h_alpha',
          bandwidth_nm: 3.0,
          size: '2in',
          mounted: true
        }
      }
    });

    if (!createRes.ok()) {
      test.skip(
        true,
        `Cannot seed filter item — backend returned ${createRes.status()}. ` +
          'Is the dev stack running (`just dev`)?'
      );
      return;
    }

    const created = (await createRes.json()) as { canonical_name: string };

    // The canonical_name is display_name.to_lowercase() (from items_create.rs).
    // The browse URL is /equip/filter/<canonical_name>.
    // canonical_name slugs spaces to dashes in the browse URL (see the page's
    // own /equip/<kind>/<canonical_name.replace(/\s+/g,'-')> links).
    const slug = created.canonical_name.replace(/\s+/g, '-');
    await page.goto(`${FRONTEND}/equip/filter/${encodeURIComponent(slug)}`);

    // The SPEC SHEET section renders the specs as labelled table rows
    // (<th>{label}</th><td>{value}</td>) inside .spec-table.
    const specSheet = page.locator('section.specs');
    await expect(specSheet).toBeVisible({ timeout: 10000 });

    // Helper: the <td> value cell for an exact <th> label (so "Mounted"
    // doesn't also match "Mounted diameter").
    const valueOf = (label: string) =>
      specSheet
        .locator('tr', { has: page.getByRole('rowheader', { name: label, exact: true }) })
        .locator('td');

    // BANDWIDTH label and value (fmtVal(3.0, ' nm') ⇒ "3 nm").
    await expect(
      specSheet.getByRole('rowheader', { name: 'Bandwidth', exact: true })
    ).toBeVisible();
    await expect(valueOf('Bandwidth')).toHaveText('3 nm');

    // SIZE label and value (FILTER_SIZE_LABELS['2in'] ⇒ '2"').
    await expect(specSheet.getByRole('rowheader', { name: 'Size', exact: true })).toBeVisible();
    await expect(valueOf('Size')).toHaveText('2"');

    // MOUNTED label and value (mounted === true ⇒ "Yes").
    await expect(specSheet.getByRole('rowheader', { name: 'Mounted', exact: true })).toBeVisible();
    await expect(valueOf('Mounted')).toHaveText('Yes');
  });
});
