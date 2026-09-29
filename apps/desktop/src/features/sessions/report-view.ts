type Translate = (key: string) => string;
type Report = Record<string, unknown> | null;

const EVENT_KEYS: Record<string, string> = {
  thermal: 'sessions.eventThermal',
  power: 'sessions.eventPower',
  platform: 'sessions.eventPlatform',
  mixed: 'sessions.eventMixed',
  turbo_end: 'sessions.eventTurboEnd',
  oem_mode_change: 'sessions.eventOemModeChange'
};

const CONFIDENCE_KEYS: Record<string, string> = {
  low: 'sessions.confidenceLow',
  medium: 'sessions.confidenceMedium',
  high: 'sessions.confidenceHigh'
};

function fill(text: string, values: Record<string, string>): string {
  return Object.entries(values).reduce(
    (result, [name, value]) => result.split(`{${name}}`).join(value),
    text
  );
}

function seconds(milliseconds: number): string {
  return String(Math.max(0, Math.round(milliseconds / 1000)));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/** One line per limit event in the report: the event's name, not its internal code, and how long it lasted. */
export function eventLines(t: Translate, report: Report): string[] {
  const events = report?.events;
  if (!Array.isArray(events)) return [];
  const lines: string[] = [];
  events.forEach((event: unknown) => {
    if (!isRecord(event) || typeof event.kind !== 'string') return;
    const key = EVENT_KEYS[event.kind];
    const name = key === undefined ? event.kind : t(key);
    const { start_ms: start, end_ms: end } = event;
    lines.push(
      typeof start === 'number' && typeof end === 'number'
        ? `${name} · ${seconds(end - start)} ${t('common.seconds')}`
        : name
    );
  });
  return lines;
}

/**
 * What the report says about how solid its own conclusion is: confidence, sensor coverage and the
 * interval it analysed, then the limit events. Read from the fields the report actually carries.
 */
export function evidenceLines(t: Translate, report: Report): string[] {
  if (report === null) return [];
  const lines: string[] = [];
  const confidence = report.confidence;
  if (typeof confidence === 'string' && CONFIDENCE_KEYS[confidence]) {
    lines.push(
      fill(t('sessions.reportConfidenceLine'), {
        level: t(CONFIDENCE_KEYS[confidence])
      })
    );
  }
  if (typeof report.coverage_tier === 'string') {
    lines.push(
      fill(t('sessions.reportCoverageLine'), { tier: report.coverage_tier })
    );
  }
  const { analyzed_start_ms: start, analyzed_end_ms: end } = report;
  if (typeof start === 'number' && typeof end === 'number' && end > start) {
    lines.push(
      fill(t('sessions.reportIntervalLine'), { seconds: seconds(end - start) })
    );
  }
  return [...lines, ...eventLines(t, report)];
}

/**
 * The "estimated impact" of the report, in words. The spec allows exactly two figures: the cooling
 * potential (a range, from the power-ceiling method) and the measured performance of the guided
 * run's own load. Nothing else is shown as a percentage. `undefined` means no figure.
 */
export function impactText(t: Translate, report: Report): string | undefined {
  if (report === null) return undefined;
  const parts: string[] = [];
  const cooling = report.cooling_potential;
  if (
    isRecord(cooling) &&
    typeof cooling.low_percent === 'number' &&
    typeof cooling.high_percent === 'number'
  ) {
    parts.push(
      fill(t('sessions.reportCoolingImpact'), {
        low: String(cooling.low_percent),
        high: String(cooling.high_percent)
      })
    );
  }
  const guided = report.guided_result;
  if (isRecord(guided) && typeof guided.relative_percent === 'number') {
    parts.push(
      fill(t('sessions.reportGuidedImpact'), {
        percent: String(Math.round(guided.relative_percent))
      })
    );
  }
  return parts.length === 0 ? undefined : parts.join(' ');
}

/** Why there is no figure: for a pure power limit the spec says cooling barely matters. */
export function noImpactReason(t: Translate, report: Report): string {
  if (report?.classification === 'power_limited') {
    return t('sessions.reportNoImpactPower');
  }
  return report?.classification === 'normal'
    ? t('sessions.reportNoImpactNormal')
    : t('sessions.reportNoImpactReason');
}

/** A test with no limitation has nothing to quantify: say so rather than "no figure available". */
export function noImpactTitle(t: Translate, report: Report): string {
  return report?.classification === 'normal'
    ? t('sessions.reportNoImpactNormalTitle')
    : t('sessions.reportNoImpact');
}
