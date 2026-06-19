import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for Astrophoto frontend.
 * The full dev stack (frontend + backend + postgres/minio/mailhog) must be
 * running. Defaults to the canonical `just dev` stack (frontend :5173). When
 * those ports are held by another local app, run astrophoto on alt ports and
 * set PLAYWRIGHT_BASE_URL (and PLAYWRIGHT_BACKEND_URL / PGPORT for the helpers).
 */
const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  // These specs drive a live, shared dev stack serially (workers:1); a single
  // SSR/upload round-trip can occasionally exceed a wait under sustained load.
  // Retries absorb such transient flakes without weakening any assertion — a
  // genuinely broken test still fails after retries.
  retries: process.env.CI ? 2 : 2,
  workers: 1,
  reporter: 'list',
  timeout: 60_000,

  use: {
    baseURL: BASE_URL,
    trace: 'on-first-retry',
    // The backend CORS origin must match. Playwright uses localhost by default.
    headless: true
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] }
    }
  ]
});
