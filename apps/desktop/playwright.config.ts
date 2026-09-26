import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  webServer: {
    command:
      'node ./node_modules/vite/bin/vite.js build --mode e2e --outDir dist-e2e && node ./node_modules/vite/bin/vite.js preview --outDir dist-e2e --host 127.0.0.1 --port 4173',
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
    // T181: there used to be a second project here called `app`, identical to `frontend` in
    // every way that matters — same Vite preview `baseURL`, same Chromium, same fake bridge from
    // `e2e/fixtures.ts`. The name promised it exercised the real application and it never did, so
    // every spec ran twice for no added coverage while nothing covered the real binary. The real
    // application is now driven by `playwright.native.config.ts` (`e2e-native/`, over CDP, with
    // its own isolated data directory); this config is the frontend harness and says so.
    {
      name: 'frontend',
      use: {
        browserName: 'chromium',
        video: 'on-first-retry',
        trace: 'retain-on-failure'
      },
      grep: /@smoke|@critical|@a11y|@visual/
    },
    {
      name: 'matrix-480x600',
      testMatch: /environment-matrix\.spec\.ts/,
      use: { browserName: 'chromium', viewport: { width: 480, height: 600 } }
    },
    {
      name: 'matrix-480x500',
      testMatch: /environment-matrix\.spec\.ts/,
      use: { browserName: 'chromium', viewport: { width: 480, height: 500 } }
    },
    {
      name: 'matrix-480x384',
      testMatch: /environment-matrix\.spec\.ts/,
      use: { browserName: 'chromium', viewport: { width: 480, height: 384 } }
    },
    {
      name: 'matrix-840x760',
      testMatch: /environment-matrix\.spec\.ts/,
      use: { browserName: 'chromium', viewport: { width: 840, height: 760 } }
    },
    {
      name: 'matrix-1100x760',
      testMatch: /environment-matrix\.spec\.ts/,
      use: { browserName: 'chromium', viewport: { width: 1100, height: 760 } }
    },
    {
      name: 'matrix-scale-200',
      testMatch: /environment-matrix\.spec\.ts/,
      use: {
        browserName: 'chromium',
        viewport: { width: 1100, height: 760 },
        deviceScaleFactor: 2
      }
    },
    {
      name: 'appearance-en',
      testMatch: /appearance-matrix\.spec\.ts/,
      use: { browserName: 'chromium', locale: 'en-US' }
    },
    {
      name: 'appearance-es',
      testMatch: /appearance-matrix\.spec\.ts/,
      use: { browserName: 'chromium', locale: 'es-ES' }
    },
    {
      name: 'a11y-en',
      testMatch: /accessibility\.spec\.ts/,
      use: { browserName: 'chromium', locale: 'en-US' }
    },
    {
      name: 'a11y-es',
      testMatch: /accessibility\.spec\.ts/,
      use: { browserName: 'chromium', locale: 'es-ES' }
    }
  ],
  use: {
    baseURL: 'http://127.0.0.1:4173',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure'
  }
});
