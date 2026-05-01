import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: {
      $lib: 'src/lib'
    },
    prerender: {
      // Phase 1: many nav targets (/targets, /signin, /about, ...) don't exist
      // yet. Warn so the build doesn't fail, fix as routes land.
      handleHttpError: 'warn',
      handleMissingId: 'warn'
    }
  }
};

export default config;
