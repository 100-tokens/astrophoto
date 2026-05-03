# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: p1-happy-path.spec.ts >> P1 happy path: signup → upload → verify → publish → permalink
- Location: tests/e2e/p1-happy-path.spec.ts:19:1

# Error details

```
Test timeout of 60000ms exceeded.
```

```
Error: page.fill: Test timeout of 60000ms exceeded.
Call log:
  - waiting for locator('input[name="display_name"]')

```

# Page snapshot

```yaml
- generic [ref=e2]:
  - generic [ref=e4]:
    - generic [ref=e5]:
      - heading "heartbit CRM" [level=1] [ref=e6]
      - paragraph [ref=e7]: Sign in to your account
    - generic [ref=e8]:
      - generic [ref=e9]:
        - generic [ref=e10]: Organization
        - textbox "Organization" [ref=e11]:
          - /placeholder: your-org
      - generic [ref=e12]:
        - generic [ref=e13]: Email
        - textbox "Email" [ref=e14]:
          - /placeholder: you@example.com
      - generic [ref=e15]:
        - generic [ref=e16]: Password
        - textbox "Password" [ref=e17]:
          - /placeholder: ••••••••
      - button "Sign In" [ref=e18]
    - paragraph [ref=e19]:
      - link "Forgot password?" [ref=e20] [cursor=pointer]:
        - /url: /reset-password
  - generic [ref=e21]: untitled page
```

# Test source

```ts
  1   | /**
  2   |  * P1 happy-path E2E test: signup → upload → verify → publish → permalink.
  3   |  *
  4   |  * Requires the full dev stack (`just dev`) running before execution:
  5   |  *   - Backend:  http://localhost:8080
  6   |  *   - Frontend: http://localhost:5173
  7   |  *   - Postgres + MinIO: running via docker compose
  8   |  *
  9   |  * Run with: cd frontend && pnpm test:e2e -- p1-happy-path
  10  |  */
  11  | 
  12  | import { test, expect } from '@playwright/test';
  13  | import { fileURLToPath } from 'url';
  14  | import path from 'path';
  15  | 
  16  | const __filename = fileURLToPath(import.meta.url);
  17  | const __dirname = path.dirname(__filename);
  18  | 
  19  | test('P1 happy path: signup → upload → verify → publish → permalink', async ({ page }) => {
  20  |   const ts = Date.now();
  21  |   // Base-36 suffix keeps the handle short and within the 30-char limit.
  22  |   const handle = `e2e${ts.toString(36).slice(-8)}`;
  23  |   const email = `e2e-${ts}@example.com`;
  24  | 
  25  |   // ── 1. Signup ─────────────────────────────────────────────────────────────
  26  | 
  27  |   await page.goto('/signup');
  28  | 
> 29  |   await page.fill('input[name="display_name"]', `E2E ${ts}`);
      |              ^ Error: page.fill: Test timeout of 60000ms exceeded.
  30  |   await page.fill('input[name="handle"]', handle);
  31  |   await page.fill('input[name="email"]', email);
  32  |   await page.fill('input[name="password"]', 'longenoughpw1');
  33  | 
  34  |   // HandlePicker debounces for 300 ms then fetches /api/auth/handle-check.
  35  |   // Wait for the availability message to appear before submitting.
  36  |   await expect(page.locator('[data-status="available"]')).toBeVisible({ timeout: 5000 });
  37  | 
  38  |   await page.click('button[type="submit"]');
  39  | 
  40  |   // Successful signup redirects to /.
  41  |   await page.waitForURL('/', { timeout: 15000 });
  42  | 
  43  |   // ── 2. Upload ──────────────────────────────────────────────────────────────
  44  | 
  45  |   await page.goto('/upload');
  46  | 
  47  |   // The file input inside UploadDropzone is hidden (display:none).
  48  |   // setInputFiles works on hidden inputs without needing to click the dropzone.
  49  |   const fixturePath = path.resolve(__dirname, 'fixtures/sample.jpg');
  50  |   await page.setInputFiles('input[type="file"]', fixturePath);
  51  | 
  52  |   // UploadFileRow renders data-state="ready" once the upload pipeline completes.
  53  |   // This can take up to ~30 s depending on thumbnail processing.
  54  |   const readyRow = page.locator('[data-state="ready"]');
  55  |   await expect(readyRow).toBeVisible({ timeout: 30000 });
  56  | 
  57  |   // The ready state shows a link to the verify step.
  58  |   const continueLink = page.locator('a:has-text("Continue to verify")');
  59  |   await expect(continueLink).toBeVisible();
  60  | 
  61  |   // ── 3. Verify (metadata) ──────────────────────────────────────────────────
  62  | 
  63  |   await continueLink.click();
  64  |   await page.waitForURL(/\/upload\/[^/]+\/verify/, { timeout: 15000 });
  65  | 
  66  |   // TargetPicker renders input[name="target"].
  67  |   await page.fill('input[name="target"]', 'M31');
  68  | 
  69  |   // CategorySegmented renders <button role="radio"> with lowercase option text.
  70  |   // 'dso' is the first non-default option; click the button containing "dso".
  71  |   await page.click('button[role="radio"]:has-text("dso")');
  72  | 
  73  |   // EquipmentAutocomplete renders input[name="camera"] / input[name="scope"].
  74  |   // Fill directly — no need to pick from the autocomplete dropdown.
  75  |   await page.fill('input[name="camera"]', 'ZWO ASI2600MC');
  76  |   await page.fill('input[name="scope"]', 'RedCat 51');
  77  | 
  78  |   // Click "Continue →" (primary submit button, not "Save as draft").
  79  |   await page.click('button[type="submit"]:has-text("Continue")');
  80  |   await page.waitForURL(/\/upload\/[^/]+\/caption/, { timeout: 15000 });
  81  | 
  82  |   // ── 4. Caption + Publish ──────────────────────────────────────────────────
  83  | 
  84  |   // The caption form has a <textarea name="caption">.
  85  |   await page.fill('textarea[name="caption"]', 'E2E caption — automated test');
  86  | 
  87  |   // "Publish" is the primary submit button on the caption page.
  88  |   await page.click('button[type="submit"]:has-text("Publish")');
  89  | 
  90  |   // After publish, SvelteKit redirects to /u/<handle>/p/<short_id>.
  91  |   await page.waitForURL(/\/u\/[^/]+\/p\/[^/]+/, { timeout: 15000 });
  92  | 
  93  |   // The photo detail page renders the target as the <h1> title.
  94  |   await expect(page.locator('h1')).toContainText('M31');
  95  | 
  96  |   // Store the canonical URL for later assertions.
  97  |   const canonicalUrl = page.url();
  98  | 
  99  |   // ── 5. Gallery ────────────────────────────────────────────────────────────
  100 | 
  101 |   await page.goto(`/u/${handle}`);
  102 | 
  103 |   // At least one grid link to this user's photo must be visible.
  104 |   const galleryLink = page.locator(`a[href*="/u/${handle}/p/"]`).first();
  105 |   await expect(galleryLink).toBeVisible({ timeout: 10000 });
  106 | 
  107 |   // Navigate back to the canonical URL to confirm it still loads correctly.
  108 |   await page.goto(canonicalUrl);
  109 |   await expect(page.locator('h1')).toContainText('M31');
  110 | });
  111 | 
```