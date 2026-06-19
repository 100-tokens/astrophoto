/**
 * P1 happy-path E2E test: signup → upload → verify → publish → permalink.
 *
 * Requires the full dev stack running before execution:
 *   - Backend:  http://localhost:8080
 *   - Frontend: http://localhost:5173
 *   - Postgres + MinIO: running via docker compose
 *
 * Run with: cd frontend && npx playwright test p1-happy-path
 */

import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';
import { FRONTEND, freshAccount, verifyEmail } from './helpers';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test('P1 happy path: signup → upload → verify → publish → permalink', async ({ page }) => {
  const acc = freshAccount(Date.now(), 'p1');

  // ── 1. Signup ─────────────────────────────────────────────────────────────

  await page.goto(`${FRONTEND}/signup`);

  await page.fill('input[name="display_name"]', acc.displayName);
  await page.fill('input[name="handle"]', acc.handle);
  await page.fill('input[name="email"]', acc.email);
  await page.fill('input[name="password"]', acc.password);

  // HandlePicker debounces for 300 ms then fetches /api/auth/handle-check.
  // Wait for the availability message to appear before submitting.
  await expect(page.locator('[data-status="available"]')).toBeVisible({ timeout: 5000 });

  await page.click('button[type="submit"]');

  // Email verification ships: signup lands on /signup/check-email, not /.
  await page.waitForURL(/\/signup\/check-email/, { timeout: 15000 });

  // Mark the account verified (the outbox link is not driven here) and sign in
  // through the UI to get a session cookie.
  verifyEmail(acc.email);
  await page.goto(`${FRONTEND}/signin`);
  await page.fill('input[name="email"]', acc.email);
  await page.fill('input[name="password"]', acc.password);
  await page.click('button[type="submit"]');
  await page.waitForURL(`${FRONTEND}/`, { timeout: 15000 });

  // ── 2. Upload ──────────────────────────────────────────────────────────────

  await page.goto('/upload');
  await page.waitForLoadState('networkidle');

  // The file input inside UploadDropzone is hidden (display:none).
  // setInputFiles works on hidden inputs without needing to click the dropzone.
  const fixturePath = path.resolve(__dirname, 'fixtures/sample.jpg');
  await page.setInputFiles('input[type="file"]', fixturePath);

  // UploadFileRow renders data-state="ready" once the upload pipeline completes.
  // This can take up to ~30 s depending on thumbnail processing.
  const readyRow = page.locator('[data-state="ready"]');
  await expect(readyRow).toBeVisible({ timeout: 30000 });

  // The footer "Verify N ready frame →" button advances to the verify step.
  const continueBtn = page.locator('button:has-text("ready frame")');
  await expect(continueBtn).toBeEnabled({ timeout: 5000 });

  // ── 3. Verify (metadata + caption + publish) ──────────────────────────────

  await continueBtn.click();
  await page.waitForURL(/\/upload\/[^/]+\/verify/, { timeout: 15000 });

  // TargetField renders a visible combobox (id="target"); free text commits to
  // the hidden input[name="target"] via Enter (commitFreetext) / blur.
  await page.fill('input#target', 'M31');
  await page.locator('input#target').press('Enter');
  await page.locator('input#target').blur();

  // CategoryRadio renders <button role="radio"> with the label "DSO".
  await page.click('button[role="radio"]:has-text("DSO")');

  // EquipmentSection renders input[name="camera"] / input[name="scope"].
  await page.fill('input[name="camera"]', 'ZWO ASI2600MC');
  await page.fill('input[name="scope"]', 'RedCat 51');

  // The caption lives on the verify form (no separate /caption step anymore).
  await page.fill('textarea[name="caption"]', 'E2E caption — automated test');

  // "Publish" is the primary submit button on the verify form for a draft.
  await page.click('button[type="submit"]:has-text("Publish")');

  // After publish, the server redirects to /photo/<id> which 301s to the
  // canonical /u/<handle>/p/<short-id> permalink.
  await page.waitForURL(/\/u\/[^/]+\/p\/[^/]+/, { timeout: 15000 });

  // The photo detail page renders the target as the <h1> title.
  await expect(page.locator('h1')).toContainText('M31');

  // Store the canonical URL for later assertions.
  const canonicalUrl = page.url();

  // ── 4. Gallery ────────────────────────────────────────────────────────────

  await page.goto(`/u/${acc.handle}`);

  // At least one grid link to this user's photo must be visible.
  const galleryLink = page.locator(`a[href*="/u/${acc.handle}/p/"]`).first();
  await expect(galleryLink).toBeVisible({ timeout: 10000 });

  // Navigate back to the canonical URL to confirm it still loads correctly.
  await page.goto(canonicalUrl);
  await expect(page.locator('h1')).toContainText('M31');
});
