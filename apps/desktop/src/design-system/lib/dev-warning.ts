/**
 * The design system's only way to warn about misuse (constitution XVII, T-LOG-012).
 *
 * `design/` cannot depend on the application's logger — it ships to the harness and the mockup
 * too — so this is the single documented exception to `no-console`: it only speaks in a
 * development build and only about how a component was called, never about user data.
 */
export function devWarning(component: string, message: string): void {
  if (!import.meta.env.DEV) return;
  // eslint-disable-next-line no-console -- the documented design-system exception, see above.
  console.warn(`[ThrottleWatch/${component}] ${message}`);
}
