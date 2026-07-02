import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173
  },
  test: {
    // Vitest owns only the colocated unit tests. Without this, its
    // default glob also collects tests/e2e/*.spec.ts and every
    // Playwright file fails at import time ("test.describe() was not
    // expected here"), breaking `pnpm test` wholesale.
    include: ['src/**/*.{test,spec}.{js,ts}']
  }
});
