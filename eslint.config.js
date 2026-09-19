import tseslint from 'typescript-eslint';

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
      'no-restricted-imports': [
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
      ],
      '@typescript-eslint/consistent-type-assertions': [
        'error',
        { assertionStyle: 'never' }
      ]
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
