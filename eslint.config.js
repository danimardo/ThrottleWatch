import svelte from 'eslint-plugin-svelte';
import tseslint from 'typescript-eslint';

// Constitution XVII: the logging rules (no `console.*`, `loglevel` only behind the wrapper) hold in
// `.svelte` components as much as in `.ts`. Only the plugin's parser setup for `.svelte` files is
// taken, not its `.svelte.ts` setup: those stay on the type-aware TypeScript parser below.
const restrictedImports = [
  'error',
  {
    patterns: [
      {
        group: ['@tauri-apps/api'],
        message: 'Importa Tauri solo desde lib/bridge.'
      },
      {
        group: ['loglevel'],
        message: 'Importa loglevel solo desde lib/logging.'
      },
      {
        group: ['design/examples/**', 'design/harness/**'],
        message: 'No importes ejemplos ni harness.'
      }
    ]
  }
];
const svelteParserSetup = svelte.configs.base
  .filter((config) => config.name !== 'svelte:base:setup-for-svelte-script')
  .map((config) =>
    config.files
      ? { ...config, files: ['apps/desktop/src/**/*.svelte'] }
      : config
  );

export default tseslint.config(
  ...tseslint.configs.strictTypeChecked.map((config) => ({
    ...config,
    files: ['apps/desktop/src/**/*.ts']
  })),
  {
    ignores: [
      'node_modules/**',
      'dist/**',
      '.tools/**',
      'apps/desktop/src-tauri/gen/**',
      'apps/desktop/src-tauri/**',
      'scripts/**'
    ]
  },
  {
    files: ['apps/desktop/src/**/*.ts'],
    languageOptions: { parserOptions: { projectService: true } },
    rules: {
      'no-console': 'error',
      'no-restricted-imports': restrictedImports,
      '@typescript-eslint/consistent-type-assertions': [
        'error',
        { assertionStyle: 'never' }
      ]
    }
  },
  ...svelteParserSetup,
  {
    files: ['apps/desktop/src/**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte']
      }
    },
    rules: {
      'no-console': 'error',
      'no-restricted-imports': restrictedImports
    }
  },
  {
    files: ['apps/desktop/src/lib/bridge/**/*.ts'],
    rules: {
      'no-restricted-imports': 'off'
    }
  },
  {
    files: ['apps/desktop/src/lib/logging/**/*.ts'],
    rules: {
      'no-restricted-imports': 'off'
    }
  }
);
