import {
  applyMotionLevel,
  type MotionLevel
} from '../../design-system/tokens/tokens';

export type ThemePreference = 'system' | 'light' | 'dark';

export function resolveTheme(
  preference: ThemePreference,
  prefersLight: boolean
): 'light' | 'dark' {
  if (preference === 'light') return 'light';
  if (preference === 'dark') return 'dark';
  return prefersLight ? 'light' : 'dark';
}

export function applyTheme(preference: ThemePreference): void {
  if (typeof document === 'undefined') return;
  const prefersLight =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-color-scheme: light)').matches;
  document.documentElement.dataset.theme = resolveTheme(
    preference,
    prefersLight
  );
}

export function applyLanguage(locale: string): void {
  if (typeof document !== 'undefined') document.documentElement.lang = locale;
}

export function applyAppearance(
  theme: ThemePreference,
  motion: MotionLevel,
  locale: string
): void {
  applyTheme(theme);
  applyMotionLevel(motion);
  applyLanguage(locale);
}

export function listenToSystemAppearance(
  theme: ThemePreference,
  onChange: () => void
): () => void {
  if (typeof window === 'undefined' || theme !== 'system')
    return () => undefined;
  const query = window.matchMedia('(prefers-color-scheme: light)');
  query.addEventListener('change', onChange);
  return () => {
    query.removeEventListener('change', onChange);
  };
}
