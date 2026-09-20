import { describe, expect, it } from 'vitest';
import { resolveGeometry, selectMonitor } from './geometry';

describe('window geometry', () => {
  const fullHd = { x: 0, y: 0, width: 1920, height: 1080 };

  it('centres the initial 1100 by 760 window', () => {
    expect(resolveGeometry(null, fullHd)).toMatchObject({
      restored_x: 410,
      restored_y: 160,
      restored_width: 1100,
      restored_height: 760,
      maximized: false,
      min_height: 600
    });
  });

  it('uses a 500 logical minimum for a 1080p display at 200 percent', () => {
    expect(resolveGeometry(null, { x: 0, y: 0, width: 960, height: 540 })).toMatchObject({
      restored_width: 960,
      restored_height: 540,
      maximized: false,
      min_width: 480,
      min_height: 500
    });
  });

  it('maximizes when the useful height is below 500 logical pixels', () => {
    expect(resolveGeometry(null, { x: 0, y: 0, width: 800, height: 480 })).toMatchObject({
      restored_width: 800,
      restored_height: 480,
      maximized: true,
      min_height: 500
    });
  });

  it('recentres a saved rectangle that is no longer usable', () => {
    expect(
      resolveGeometry(
        {
          restored_x: 4000,
          restored_y: 4000,
          restored_width: 840,
          restored_height: 600,
          maximized: false
        },
        fullHd
      )
    ).toMatchObject({ restored_x: 540, restored_y: 240 });
  });

  it('restores on the saved monitor when it is still available', () => {
    const primary = { fingerprint: 'primary', workArea: fullHd };
    const secondary = {
      fingerprint: 'secondary',
      workArea: { x: 1920, y: 0, width: 1280, height: 1024 }
    };

    expect(
      selectMonitor(
        {
          restored_x: 1950,
          restored_y: 80,
          restored_width: 840,
          restored_height: 600,
          maximized: false,
          display_fingerprint: 'secondary'
        },
        [primary, secondary],
        primary
      )
    ).toBe(secondary);
  });

  it('uses a visible monitor after the saved monitor disappears', () => {
    const primary = { fingerprint: 'primary', workArea: fullHd };
    const secondary = {
      fingerprint: 'renamed-secondary',
      workArea: { x: 1920, y: 0, width: 1280, height: 1024 }
    };

    expect(
      selectMonitor(
        {
          restored_x: 1950,
          restored_y: 80,
          restored_width: 840,
          restored_height: 600,
          maximized: false,
          display_fingerprint: 'old-secondary'
        },
        [primary, secondary],
        primary
      )
    ).toBe(secondary);
  });

  it('keeps the current monitor when no saved rectangle selects another one', () => {
    const current = { fingerprint: 'current', workArea: fullHd };
    const other = {
      fingerprint: 'other',
      workArea: { x: 1920, y: 0, width: 1280, height: 1024 }
    };

    expect(selectMonitor(null, [other], current)).toBe(current);
  });
});
