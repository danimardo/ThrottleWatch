/**
 * Single source of truth for `diagnostic_report.classification`.
 *
 * StatusChip and StatusHero both read this table instead of each
 * choosing their own color/icon — that is the whole point of it.
 * The system's own rule (see components/StatusChip/README in the
 * design system artifact) is: "cambiar el color de una clasificación
 * en un solo lugar de la app y no en otro rompe el principio de «el
 * mismo color representa la misma clase en toda la app»." If you add
 * a ninth classification, add it here first and both components
 * pick it up automatically.
 */

export type Classification =
  | 'normal'
  | 'hot_unproven'
  | 'thermal_probable'
  | 'thermal_confirmed'
  | 'power_limited'
  | 'platform_limited'
  | 'mixed_limit'
  | 'indeterminate';

export type StatusIconKind = 'check' | 'warning' | 'flame' | 'bolt' | 'device' | 'mixed' | 'unknown';

export interface ClassificationMeta {
  /** CSS color token name (without --), e.g. "status-thermal". */
  token: string;
  /** Tinted-background opacity over `surface`, 0–1. */
  bgOpacity: number;
  icon: StatusIconKind;
  /**
   * mixed_limit is deliberately neutral in the pill background (never
   * a third invented color) — it renders on surface-raised/text-primary
   * instead of a tinted status color.
   */
  neutralBg?: boolean;
}

export const CLASSIFICATION_META: Record<Classification, ClassificationMeta> = {
  normal: { token: 'status-normal', bgOpacity: 0.12, icon: 'check' },
  hot_unproven: { token: 'status-warm', bgOpacity: 0.12, icon: 'warning' },
  // thermal_probable and thermal_confirmed share status-thermal and differ
  // ONLY by background opacity (10% vs 16%) — never by a second color.
  thermal_probable: { token: 'status-thermal', bgOpacity: 0.10, icon: 'flame' },
  thermal_confirmed: { token: 'status-thermal', bgOpacity: 0.16, icon: 'flame' },
  power_limited: { token: 'status-power', bgOpacity: 0.12, icon: 'bolt' },
  // Limited by the device, not by the CPU's own thermal control: the
  // manufacturer lowers the power limit as the chassis heats up
  // (chassis_thermal), or an external PROCHOT (external_prochot). Warm,
  // not thermal red, and its own "device" icon so it never reads as
  // either a CPU-thermal or a plain power limitation.
  platform_limited: { token: 'status-warm', bgOpacity: 0.16, icon: 'device' },
  mixed_limit: { token: 'text-primary', bgOpacity: 0, icon: 'mixed', neutralBg: true },
  indeterminate: { token: 'status-unknown', bgOpacity: 0.12, icon: 'unknown' }
};

/**
 * `indeterminate` (no evidence) and `normal` (evidence of no limit) are
 * semantic opposites. Never render them side by side as if they were
 * two points on the same scale without distinguishing which is the
 * active state — that is exactly the observation/inference/confirmation
 * confusion the product constitution exists to prevent.
 */
export const OPPOSED_CLASSIFICATIONS: [Classification, Classification] = ['normal', 'indeterminate'];
