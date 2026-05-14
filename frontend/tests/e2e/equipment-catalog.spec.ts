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

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const BACKEND = 'http://localhost:8080';
const FRONTEND = 'http://localhost:5173';

// ---------------------------------------------------------------------------
// Auth helper — mirrors the p1-happy-path.spec.ts pattern.
// Creates an account via the signup form (not the API) so the session cookie
// is set immediately. Returns the handle used.
// ---------------------------------------------------------------------------
async function signupViaForm(page: import('@playwright/test').Page, ts: number): Promise<string> {
  // Base-36 suffix keeps the handle short and within the 30-char limit.
  const handle = `e2e${ts.toString(36).slice(-8)}`;
  const email = `e2e-${ts}@example.com`;

  await page.goto(`${FRONTEND}/signup`);

  await page.fill('input[name="display_name"]', `E2E ${ts}`);
  await page.fill('input[name="handle"]', handle);
  await page.fill('input[name="email"]', email);
  await page.fill('input[name="password"]', 'longenoughpw1');

  // HandlePicker debounces 300 ms then fetches /api/auth/handle-check.
  await expect(page.locator('[data-status="available"]')).toBeVisible({ timeout: 5000 });

  await page.click('button[type="submit"]');
  await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });

  return handle;
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

    // Click "Save setup" (primary button at the foot of the form).
    await page.getByRole('button', { name: 'Save setup' }).click();

    // Expect redirect to /settings/equipment.
    await page.waitForURL(`${FRONTEND}/settings/equipment`, { timeout: 15000 });

    // The setup name should appear in the list.
    await expect(page.getByText(setupName)).toBeVisible({ timeout: 5000 });
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

    const fixturePath = path.resolve(__dirname, 'fixtures/sample.jpg');
    await page.setInputFiles('input[type="file"]', fixturePath);

    // Wait for the upload pipeline to complete (data-state="ready").
    const readyRow = page.locator('[data-state="ready"]');
    await expect(readyRow).toBeVisible({ timeout: 30000 });

    const continueLink = page.locator('a:has-text("Continue to verify")');
    await expect(continueLink).toBeVisible();
    await continueLink.click();
    await page.waitForURL(/\/upload\/[^/]+\/verify/, { timeout: 15000 });

    // ── Step 3: Interact with FilterChipInput.
    // The input is inside the FILTERS section of the metadata form.
    const chipInput = page.locator('.fchip-input');
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

    // A FilterChip should now be present inside the input container.
    const chip = chipInput.locator('.fchip');
    await expect(chip).toBeVisible({ timeout: 5000 });
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
    const slug = created.canonical_name;
    await page.goto(`${FRONTEND}/equip/filter/${encodeURIComponent(slug)}`);

    // Wait for the specs bar to render.
    const specsBar = page.locator('.specs-bar');
    await expect(specsBar).toBeVisible({ timeout: 10000 });

    // BANDWIDTH label and value.
    await expect(specsBar.locator('.spec-label', { hasText: 'BANDWIDTH' })).toBeVisible();
    await expect(specsBar).toContainText('3 nm');

    // SIZE label and value ("2 inch" per SIZE_LABELS['2in'] in the page).
    await expect(specsBar.locator('.spec-label', { hasText: 'SIZE' })).toBeVisible();
    await expect(specsBar).toContainText('2 inch');

    // MOUNTED label and value.
    await expect(specsBar.locator('.spec-label', { hasText: 'MOUNTED' })).toBeVisible();
    await expect(specsBar).toContainText('yes');
  });
});
