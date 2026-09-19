import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  webServer: {
    command:
      'pnpm build && pnpm exec vite preview --host 127.0.0.1 --port 4173',
    port: 4173,
    reuseExistingServer: true,
    env: { FORCE_COLOR: '0', NO_COLOR: '1' }
  },
  fullyParallel: true,
  reporter: [
    ['list'],
    ['html', { outputFolder: 'playwright-report', open: 'never' }]
  ],
  projects: [
    {
      name: 'frontend',
      use: { browserName: 'chromium' },
      grep: /@smoke|@critical|@a11y|@visual/
    },
    {
      name: 'app',
      use: {
        browserName: 'chromium',
        video: 'on-first-retry',
        trace: 'retain-on-failure'
      },
      grep: /@smoke|@critical|@a11y|@visual/
    }
  ],
  use: {
    baseURL: 'http://127.0.0.1:4173',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure'
  }
});
