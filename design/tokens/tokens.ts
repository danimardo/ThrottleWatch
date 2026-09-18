/**
 * ThrottleWatch — design tokens as typed data.
 *
 * This mirrors tokens.css exactly, so code that needs a
 * raw value (a chart, a canvas draw, a computed style) never hardcodes
 * a hex value — it reads it from here. UI markup should still prefer
 * the CSS custom properties (var(--status-thermal)) so it repaints on
 * theme switch for free; reach for this module only where CSS can't
 * reach (canvas, SVG generated off-DOM, tests).
 */

export type Theme = 'dark' | 'light';

export const THEMES: { id: Theme; name: string }[] = [
  { id: 'dark', name: 'Oscuro' },
  { id: 'light', name: 'Claro' }
];

export interface ColorToken {
  name: string;
  value: Record<Theme, string>;
  usage: string;
}

export const COLOR_TOKENS: ColorToken[] = [
  { name: 'bg', value: { dark: '#101114', light: '#F2F4F7' }, usage: 'Fondo de la ventana y de las áreas con scroll.' },
  { name: 'surface', value: { dark: '#1C1C1E', light: '#FFFFFF' }, usage: 'Superficie de contenedores agrupados.' },
  { name: 'surface-raised', value: { dark: '#242426', light: '#EBEFF5' }, usage: 'Tiles de icono y hover ligero de filas de navegación.' },
  { name: 'surface-sunken', value: { dark: '#2C2C2E', light: '#E1E7F0' }, usage: 'Pista de fondo de anillos y barras de progreso.' },
  { name: 'hairline', value: { dark: 'rgba(255,255,255,0.14)', light: 'rgba(29,29,31,0.12)' }, usage: 'Único recurso de separación entre regiones. Nunca sombra.' },
  { name: 'text-primary', value: { dark: '#FFFFFF', light: '#1D1D1F' }, usage: 'Texto principal.' },
  { name: 'text-secondary', value: { dark: '#98989D', light: '#4B4B50' }, usage: 'Texto de apoyo.' },
  { name: 'text-tertiary', value: { dark: '#67676C', light: '#7A7A80' }, usage: 'Micro-texto secundario. Evitar para frases completas.' },
  { name: 'accent-blue', value: { dark: '#0A84FF', light: '#0066CC' }, usage: 'Color de sistema: navegación activa, foco, toolbar, cifras neutras. Nunca para un estado de alerta.' },
  { name: 'status-normal', value: { dark: '#32D74B', light: '#158237' }, usage: 'Clasificación "normal": sin limitación detectada.' },
  { name: 'status-warm', value: { dark: '#FF9F0A', light: '#B25000' }, usage: 'Clasificación "hot_unproven".' },
  { name: 'status-thermal', value: { dark: '#FF453A', light: '#D70015' }, usage: 'Clasificaciones "thermal_probable" / "thermal_confirmed".' },
  { name: 'status-power', value: { dark: '#BF5AF2', light: '#9433C7' }, usage: 'Clasificación "power_limited".' },
  { name: 'status-unknown', value: { dark: '#98989D', light: '#57575C' }, usage: 'Clasificación "indeterminate". No combinar con status-normal en el mismo contexto.' }
] as const;

export const SPACING_TOKENS = {
  'space-1': '4px',
  'space-2': '8px',
  'space-3': '12px',
  'space-4': '16px',
  'space-5': '20px',
  'space-6': '24px',
  'space-8': '32px',
  'space-10': '40px'
} as const;

export const RADIUS_TOKENS = {
  'radius-sm': '8px',
  'radius-md': '12px',
  'radius-lg': '14px',
  'radius-full': '999px'
} as const;

export const FONT_SANS =
  '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif';

export type TypeStyleName =
  | 'value-xl' | 'value-lg' | 'value-md'
  | 'body' | 'body-strong' | 'label' | 'caption' | 'tag';

export const TYPE_STYLES: Record<TypeStyleName, { fontSize: string; lineHeight: string; fontWeight: number; letterSpacing?: string }> = {
  'value-xl': { fontSize: '32px', lineHeight: '38px', fontWeight: 700 },
  'value-lg': { fontSize: '24px', lineHeight: '29px', fontWeight: 700 },
  'value-md': { fontSize: '17px', lineHeight: '22px', fontWeight: 700 },
  body: { fontSize: '13px', lineHeight: '18px', fontWeight: 400 },
  'body-strong': { fontSize: '13px', lineHeight: '18px', fontWeight: 650 },
  label: { fontSize: '11px', lineHeight: '14px', fontWeight: 650, letterSpacing: '0.01em' },
  caption: { fontSize: '10px', lineHeight: '13px', fontWeight: 600, letterSpacing: '0.04em' },
  tag: { fontSize: '11px', lineHeight: '14px', fontWeight: 700, letterSpacing: '0.01em' }
};

/**
 * Width tiers for the "compactarse y estirarse" layout rules (see
 * README). < compact -> BottomBar; < expanded -> icon-only rail;
 * >= expanded -> full sidebar + causal rail visible.
 */
export const BREAKPOINTS = {
  compact: 700,
  expanded: 980
} as const;

/** Tone -> color token, shared by StatWidget and CausalRail tiles. */
export type Tone = 'accent' | 'thermal' | 'warm' | 'normal' | 'power' | 'unknown';

export const TONE_TOKENS: Record<Tone, string> = {
  accent: 'accent-blue',
  thermal: 'status-thermal',
  warm: 'status-warm',
  normal: 'status-normal',
  power: 'status-power',
  unknown: 'status-unknown'
};

/**
 * Glass material and motion tokens (2026-09-18). Same values as
 * tokens.css; the CSS is the runtime source, this is for non-CSS
 * contexts (tests, docs, a future canvas renderer).
 */
export type GlassLevel = 'full' | 'reduced' | 'off';
export type MotionLevel = 'system' | 'reduced' | 'full';

export const GLASS = {
  blurPx: 18,
  saturate: 1.35,
  alpha: { base: 0.8, strong: 0.9, subtle: 0.62 },
  alphaLight: { base: 0.76, strong: 0.88, subtle: 0.58 },
  alphaReduced: { base: 0.92, strong: 0.96, subtle: 0.8 },
  levels: ['full', 'reduced', 'off'] as const
} as const;

export const MOTION = {
  fastMs: 160,
  baseMs: 240,
  slowMs: 420,
  ambientMs: 48000,
  staggerMs: 45,
  easeOut: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
  easeInOut: 'cubic-bezier(0.65, 0, 0.35, 1)',
  spring: 'cubic-bezier(0.34, 1.45, 0.64, 1)'
} as const;

/** Mirror the app preferences onto <html> so tokens.css can react. */
export function applyGlassLevel(level: GlassLevel): void {
  if (typeof document === 'undefined') return;
  if (level === 'full') delete document.documentElement.dataset.glass;
  else document.documentElement.dataset.glass = level;
}
export function applyMotionLevel(level: MotionLevel): void {
  if (typeof document === 'undefined') return;
  if (level === 'system') delete document.documentElement.dataset.motion;
  else document.documentElement.dataset.motion = level;
}
/** True when animations should be skipped (OS preference or app preference). */
export function motionIsReduced(): boolean {
  if (typeof document === 'undefined' || typeof window === 'undefined') return false;
  const pref = document.documentElement.dataset.motion;
  if (pref === 'reduced') return true;
  if (pref === 'full') return false;
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
}

