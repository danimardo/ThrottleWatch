import { Resvg } from '@resvg/resvg-js';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

// Same "anillo parcial" motif as design/brand/app-icon (track + accent circle), adapted for
// 16-32px tray sizes: a full ring (not a partial arc — the tray states are not a live percentage)
// with a state-specific shape mark so no state depends on color alone (design system rule 5).
// State colors are the exact hex values in apps/desktop/src-tauri/src/tray.rs::TrayIconState::color
// so the tray glyph and the corner dot it replaces always agree.

const STATES = {
  normal: { color: '#2e9e5b', mark: 'none' },
  warning: { color: '#e0a100', mark: 'dot-small' },
  critical: { color: '#d13438', mark: 'dot-large' },
  unknown: { color: '#8a8a8a', mark: 'dashed' },
  disconnected: { color: '#8a8a8a', mark: 'slash' }
};

// Neutral tone for the non-accent strokes (Unknown's dashed ring, Disconnected's ring+slash):
// light-on-dark for the `dark` (dark-taskbar) variant, dark-on-light for the `light` variant.
// The three colored states (normal/warning/critical) use their own hex in both variants — mid
// green/amber/red already read on both a light and a dark taskbar.
const VARIANTS = {
  dark: { neutral: '#E5E5E7' },
  light: { neutral: '#3A3A3C' }
};

const SIZE = 32;
const CENTER = SIZE / 2;
const RADIUS = 11;
const STROKE = 4;

function ring(color, dashed = false) {
  const dash = dashed ? ` stroke-dasharray="3 3.2"` : '';
  return `<circle cx="${CENTER}" cy="${CENTER}" r="${RADIUS}" fill="none" stroke="${color}" stroke-width="${STROKE}"${dash} />`;
}

function dot(radius) {
  return `<circle cx="${CENTER}" cy="${CENTER}" r="${radius}" fill="currentColor" />`;
}

function slash(color) {
  const offset = RADIUS - STROKE / 2 + 1;
  const x1 = CENTER - offset * Math.SQRT1_2;
  const y1 = CENTER - offset * Math.SQRT1_2;
  const x2 = CENTER + offset * Math.SQRT1_2;
  const y2 = CENTER + offset * Math.SQRT1_2;
  return `<line x1="${x1.toFixed(2)}" y1="${y1.toFixed(2)}" x2="${x2.toFixed(2)}" y2="${y2.toFixed(2)}" stroke="${color}" stroke-width="${STROKE - 1}" stroke-linecap="round" />`;
}

function buildSvg(stateKey, variantKey) {
  const state = STATES[stateKey];
  const variant = VARIANTS[variantKey];
  const isNeutralRing = stateKey === 'unknown' || stateKey === 'disconnected';
  const ringColor = isNeutralRing ? variant.neutral : state.color;
  const parts = [ring(ringColor, state.mark === 'dashed')];

  if (state.mark === 'dot-small') {
    parts.push(`<g color="${state.color}">${dot(2.6)}</g>`);
  } else if (state.mark === 'dot-large') {
    parts.push(`<g color="${state.color}">${dot(5)}</g>`);
  } else if (state.mark === 'slash') {
    parts.push(slash(variant.neutral));
  }

  return `<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}" xmlns="http://www.w3.org/2000/svg">\n  ${parts.join('\n  ')}\n</svg>\n`;
}

const SIZES = [16, 20, 24, 32, 48];

async function main() {
  const outDir = process.argv[2];
  await mkdir(outDir, { recursive: true });
  for (const stateKey of Object.keys(STATES)) {
    for (const variantKey of Object.keys(VARIANTS)) {
      const name = `${stateKey}-${variantKey}`;
      const svg = buildSvg(stateKey, variantKey);
      await writeFile(join(outDir, `${name}.svg`), svg, 'utf8');
      for (const size of SIZES) {
        const resvg = new Resvg(svg, { fitTo: { mode: 'width', value: size } });
        const png = resvg.render().asPng();
        await writeFile(join(outDir, `${name}-${size}.png`), png);
      }
      console.log(`generated ${name}`);
    }
  }
}

main();
