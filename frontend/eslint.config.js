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
      'src/lib/components/profile/editor/SocialLinksSection.svelte'
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
