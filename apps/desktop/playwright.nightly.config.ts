import { defineConfig } from '@playwright/test';

const sizes = [
  ['480x600', 480, 600],
  ['480x500', 480, 500],
  ['480x384-maximized', 480, 384],
  ['840x760', 840, 760],
  ['1100x760', 1100, 760]
] as const;
const themes = ['light', 'dark'] as const;
const locales = ['es-ES', 'en-US'] as const;
const scales = [1, 1.25, 1.5, 2] as const;
const forcedColors = ['none', 'active'] as const;
const reducedMotion = ['no-preference', 'reduce'] as const;
const glassModes = ['full', 'off'] as const;

const projects = themes.flatMap((theme) =>
  locales.flatMap((locale) =>
    sizes.flatMap(([size, width, height]) =>
      scales.flatMap((scale) =>
        forcedColors.flatMap((forcedColor) =>
          reducedMotion.flatMap((motion) =>
            glassModes.map((glass) => ({
              name: `${theme}-${locale}-${size}-x${scale}-${forcedColor}-${motion}-${glass}`,
              testMatch: /environment-matrix\.spec\.ts/,
              metadata: { glass },
              use: {
                browserName: 'chromium' as const,
                colorScheme: theme,
                locale,
                viewport: { width, height },
                deviceScaleFactor: scale,
                forcedColors: forcedColor,
                reducedMotion: motion
              }
            }))
          )
        )
      )
    )
  )
);

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
  reporter: [['list']],
  repeatEach: 3,
  projects,
  use: {
    baseURL: 'http://127.0.0.1:4173',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure'
  }
});
