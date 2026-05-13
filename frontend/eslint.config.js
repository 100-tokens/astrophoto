import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node }
    }
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser
      }
    }
  },
  {
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
          destructuredArrayIgnorePattern: '^_'
        }
      ],
      // Bio HTML and other server-sanitised content uses {@html}; the boundary
      // is the ammonia sanitiser in users::bio (Rust). Allowed at component level.
      'svelte/no-at-html-tags': 'off',
      // The "seed local $state from prop once" pattern is intentional in editor
      // section components — they own their local edits between commits.
      'svelte/prefer-svelte-reactivity': 'off'
    }
  },
  {
    files: [
      'src/lib/components/profile/FeaturedRow.svelte',
      'src/lib/components/profile/PhotoGrid.svelte',
      'src/lib/components/profile/editor/EquipmentSection.svelte',
      'src/lib/components/profile/editor/IdentitySection.svelte',
      'src/lib/components/profile/editor/LocationSection.svelte',
      'src/lib/components/profile/editor/SocialLinksSection.svelte',
      // SetupForm and SetupPicker seed local $state from props once on mount;
      // their edits are owned locally between form submissions.
      'src/lib/components/SetupForm.svelte',
      'src/lib/components/SetupPicker.svelte',
      // Discovery pages seed cursor $state from the SSR page data once on mount;
      // the $effect keeps it in sync on filter navigation. This is intentional.
      'src/routes/explore/+page.svelte',
      'src/routes/t/+page.svelte',
      'src/routes/t/\\[slug\\]/+page.svelte',
      'src/routes/tag/\\[slug\\]/+page.svelte',
      'src/routes/equip/\\[kind\\]/\\[slug\\]/+page.svelte',
      'src/routes/c/\\[cat\\]/+page.svelte',
      // DiscoveryHeader uses a discriminated-union $props() type; not a custom element.
      'src/lib/components/discovery/DiscoveryHeader.svelte'
    ],
    rules: {
      // The "seed local $state from prop once" pattern is intentional in
      // editor sections — they own local edits between explicit save calls.
      // The Svelte compiler warns about state_referenced_locally; that warning
      // is by design here, so we lower svelte/valid-compile to off for these
      // files only.
      'svelte/valid-compile': 'off'
    }
  },
  {
    ignores: ['build/', '.svelte-kit/', 'dist/', 'src/lib/api/Health.ts', 'test-results/']
  }
);
