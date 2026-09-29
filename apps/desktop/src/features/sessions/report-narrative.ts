import type { Tone } from '../../design-system/tokens/tokens';

type Translate = (key: string) => string;
type Report = Record<string, unknown> | null;

export type ChainKind = 'load' | 'temperature' | 'power' | 'platform' | 'clock';

export interface ChainNode {
  id: string;
  kind: ChainKind;
  label: string;
  value: string;
  tone: Tone;
}

export interface ReportNarrative {
  observed: string;
  alternativeCauses: string[];
  cannotConclude: string[];
  recommendations: string[];
  causalChain: ChainNode[] | undefined;
}

const CLASSES = [
  'normal',
  'hot_unproven',
  'thermal_probable',
  'thermal_confirmed',
  'power_limited',
  'platform_limited',
  'mixed_limit',
  'indeterminate'
] as const;
type Class = (typeof CLASSES)[number];

function classOf(report: Report): Class | null {
  return CLASSES.find((item) => item === report?.classification) ?? null;
}

function hasEvent(report: Report, kind: string): boolean {
  const events = report?.events;
  return (
    Array.isArray(events) &&
    events.some(
      (event: unknown) =>
        typeof event === 'object' &&
        event !== null &&
        'kind' in event &&
        event.kind === kind
    )
  );
}

/**
 * What the report says in words, per limiting class, from the classification table in the spec
 * (rules 1–10) and the coverage tier the session had. Nothing here is computed from the samples: it
 * is what each class *means* and what it does not allow to conclude, so it only ever states what
 * the class itself guarantees.
 *
 * The causal chain is drawn only when the class rests on direct reasons from the processor
 * (coverage tier A), and never with the end of turbo as a link; below tier A the sequence is an
 * inference and is not drawn.
 */
/**
 * `advancedAccess` is this machine's *current* advanced-access state (`get_coverage`), used only
 * to decide whether recommending it makes sense — never to judge the session itself, which stays
 * governed entirely by `report.coverage_tier` (a past session may have run under different
 * conditions, or on a different machine entirely for an imported one). `undefined`/`null` (not
 * fetched yet, or truly unknown) keeps the recommendation showing, same as before this parameter
 * existed.
 */
export function narrativeFor(
  t: Translate,
  report: Report,
  advancedAccess?: string | null
): ReportNarrative {
  const cls = classOf(report);
  const tier =
    typeof report?.coverage_tier === 'string' ? report.coverage_tier : null;
  const direct = tier === 'A';
  const inferred = tier === 'B' || tier === 'C';
  const oem = hasEvent(report, 'oem_mode_change');
  // Found 2026-09-28: this recommendation used to fire on every inferred verdict, even with
  // advanced access already active — permanently on AMD, since level A is not reachable there yet
  // (`spec.md` §421) and every session is `inferred` regardless of how well access works.
  const advancedAccessAlreadyActive =
    advancedAccess === 'available' ||
    advancedAccess === 'capped_by_vendor' ||
    advancedAccess === 'not_needed';
  const key = (name: string): string => t(`reportNarrative.${name}`);

  const alternativeCauses: string[] = [];
  const cannotConclude: string[] = [];
  const recommendations: string[] = [];

  if (cls === 'thermal_probable')
    alternativeCauses.push(key('altSimultaneousPower'));
  if (cls === 'indeterminate') {
    alternativeCauses.push(key('altWindowsPower'));
    if (oem) alternativeCauses.push(key('altOemMode'));
  }

  if (cls !== null && inferred && cls !== 'indeterminate') {
    cannotConclude.push(key('noDirectReasons'));
  }
  if (cls === 'thermal_probable') cannotConclude.push(key('noPowerLimitProof'));
  if (cls === 'power_limited') cannotConclude.push(key('noCoolingGain'));
  if (cls === 'hot_unproven') cannotConclude.push(key('hotIsNotALimit'));
  if (cls === 'indeterminate') cannotConclude.push(key('noSingleCause'));
  if (cls === 'normal') cannotConclude.push(key('normalIsNotAGuarantee'));

  if (
    cls === 'thermal_confirmed' ||
    cls === 'thermal_probable' ||
    cls === 'mixed_limit'
  ) {
    recommendations.push(key('recCooling'));
  }
  if (cls === 'power_limited' || cls === 'mixed_limit') {
    recommendations.push(key('recPowerLimits'));
  }
  if (cls === 'platform_limited') recommendations.push(key('recPlatform'));
  if (cls === 'indeterminate') {
    recommendations.push(key('recRepeatSustained'));
    recommendations.push(key('recWindowsPower'));
    if (oem) recommendations.push(key('recNoVentilationForOem'));
  }
  if (cls === 'hot_unproven') recommendations.push(key('recRepeatSustained'));
  if (cls !== null && inferred && !advancedAccessAlreadyActive) {
    recommendations.push(key('recAdvancedAccess'));
  }

  return {
    observed:
      cls === null ? t('sessions.reportObserved') : key(`observed_${cls}`),
    alternativeCauses,
    cannotConclude,
    recommendations,
    causalChain: direct ? chainFor(t, cls, report) : undefined
  };
}

function chainFor(
  t: Translate,
  cls: Class | null,
  report: Report
): ChainNode[] | undefined {
  const key = (name: string): string => t(`reportNarrative.${name}`);
  const load: ChainNode = {
    id: 'load',
    kind: 'load',
    label: key('chainLoad'),
    value: key('chainLoadValue'),
    tone: 'accent'
  };
  const temperature: ChainNode = {
    id: 'temperature',
    kind: 'temperature',
    label: key('chainTemperature'),
    value: key('chainAtLimit'),
    tone: 'thermal'
  };
  const power: ChainNode = {
    id: 'power',
    kind: 'power',
    label: key('chainPower'),
    value: key('chainAtLimit'),
    tone: 'power'
  };
  const platform: ChainNode = {
    id: 'platform',
    kind: 'platform',
    label: key('chainPlatform'),
    value: key('chainPlatformValue'),
    tone: 'warm'
  };
  const belowBase = report?.severity === 'below_base';
  const clockTone: Tone =
    cls === 'power_limited'
      ? 'power'
      : cls === 'platform_limited'
        ? 'warm'
        : 'thermal';
  const clock: ChainNode = {
    id: 'clock',
    kind: 'clock',
    label: key('chainClock'),
    value: belowBase ? key('chainBelowBase') : key('chainReduced'),
    tone: clockTone
  };
  switch (cls) {
    case 'thermal_confirmed':
      return [load, temperature, clock];
    case 'power_limited':
      return [load, power, clock];
    case 'platform_limited':
      return [load, platform, clock];
    case 'mixed_limit':
      return [load, temperature, power, clock];
    default:
      return undefined;
  }
}
