/**
 * P1 happy-path E2E test: signup → upload → verify → publish → permalink.
 *
 * Requires the full dev stack (`just dev`) running before execution:
 *   - Backend:  http://localhost:8080
 *   - Frontend: http://localhost:5173
 *   - Postgres + MinIO: running via docker compose
 *
 * Run with: cd frontend && pnpm test:e2e -- p1-happy-path
 */

import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test('P1 happy path: signup → upload → verify → publish → permalink', async ({ page }) => {
  const ts = Date.now();
  // Base-36 suffix keeps the handle short and within the 30-char limit.
  const handle = `e2e${ts.toString(36).slice(-8)}`;
  const email = `e2e-${ts}@example.com`;

  // ── 1. Signup ─────────────────────────────────────────────────────────────

  await page.goto('/signup');

  await page.fill('input[name="display_name"]', `E2E ${ts}`);
  await page.fill('input[name="handle"]', handle);
  await page.fill('input[name="email"]', email);
  await page.fill('input[name="password"]', 'longenoughpw1');

  // HandlePicker debounces for 300 ms then fetches /api/auth/handle-check.
  // Wait for the availability message to appear before submitting.
  await expect(page.locator('[data-status="available"]')).toBeVisible({ timeout: 5000 });

  await page.click('button[type="submit"]');

  // Successful signup redirects to /.
  await page.waitForURL('/', { timeout: 15000 });

  // ── 2. Upload ──────────────────────────────────────────────────────────────

  await page.goto('/upload');

  // The file input inside UploadDropzone is hidden (display:none).
  // setInputFiles works on hidden inputs without needing to click the dropzone.
  const fixturePath = path.resolve(__dirname, 'fixtures/sample.jpg');
  await page.setInputFiles('input[type="file"]', fixturePath);

  // UploadFileRow renders data-state="ready" once the upload pipeline completes.
  // This can take up to ~30 s depending on thumbnail processing.
  const readyRow = page.locator('[data-state="ready"]');
  await expect(readyRow).toBeVisible({ timeout: 30000 });

  // The ready state shows a link to the verify step.
  const continueLink = page.locator('a:has-text("Continue to verify")');
  await expect(continueLink).toBeVisible();

  // ── 3. Verify (metadata) ──────────────────────────────────────────────────

  await continueLink.click();
  await page.waitForURL(/\/upload\/[^/]+\/verify/, { timeout: 15000 });

  // TargetPicker renders input[name="target"].
  await page.fill('input[name="target"]', 'M31');

  // CategorySegmented renders <button role="radio"> with lowercase option text.
  // 'dso' is the first non-default option; click the button containing "dso".
  await page.click('button[role="radio"]:has-text("dso")');

  // EquipmentAutocomplete renders input[name="camera"] / input[name="scope"].
  // Fill directly — no need to pick from the autocomplete dropdown.
  await page.fill('input[name="camera"]', 'ZWO ASI2600MC');
  await page.fill('input[name="scope"]', 'RedCat 51');

  // Click "Continue →" (primary submit button, not "Save as draft").
  await page.click('button[type="submit"]:has-text("Continue")');
  await page.waitForURL(/\/upload\/[^/]+\/caption/, { timeout: 15000 });

  // ── 4. Caption + Publish ──────────────────────────────────────────────────

  // The caption form has a <textarea name="caption">.
  await page.fill('textarea[name="caption"]', 'E2E caption — automated test');

  // "Publish" is the primary submit button on the caption page.
  await page.click('button[type="submit"]:has-text("Publish")');

  // After publish, SvelteKit redirects to /u/<handle>/p/<short_id>.
  await page.waitForURL(/\/u\/[^/]+\/p\/[^/]+/, { timeout: 15000 });

  // The photo detail page renders the target as the <h1> title.
  await expect(page.locator('h1')).toContainText('M31');

  // Store the canonical URL for later assertions.
  const canonicalUrl = page.url();

  // ── 5. Gallery ────────────────────────────────────────────────────────────

  await page.goto(`/u/${handle}`);

  // At least one grid link to this user's photo must be visible.
  const galleryLink = page.locator(`a[href*="/u/${handle}/p/"]`).first();
  await expect(galleryLink).toBeVisible({ timeout: 10000 });

  // Navigate back to the canonical URL to confirm it still loads correctly.
  await page.goto(canonicalUrl);
  await expect(page.locator('h1')).toContainText('M31');
});
