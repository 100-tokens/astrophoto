import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for Astrophoto frontend.
 * The full dev stack (backend on :8080, frontend on :5173) must be running.
 * Start it with `just dev` before running `pnpm test:e2e`.
 */
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: 'list',
  timeout: 60_000,

  use: {
    baseURL: 'http://localhost:5173',
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
