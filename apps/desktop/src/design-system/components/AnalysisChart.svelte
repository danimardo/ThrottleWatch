<script lang="ts">
  /**
   * The synced multi-track chart at the heart of Análisis: temperatura,
   * frecuencia activa, carga, and potencia sharing one time axis, one
   * cursor, and one event-band layer — all hand-rolled SVG. See this
   * package's AGENTS.md ("AnalysisChart: SVG vs. ECharts") for why this
   * stays a hand-rolled SVG rather than an ECharts wrapper, and the
   * scale limits (max points per track, max total points) that
   * decision is verified against.
   *
   * A `null` value is a REAL gap (the sensor didn't report that
   * sample) and is never bridged by interpolation — each track's line
   * breaks into separate path segments around a gap. A sample flagged
   * `reducedQuality` draws with a dashed stroke instead of a solid
   * one, so reduced confidence reads as a distinct pattern, not just a
   * dimmer or different color (same principle `CoreCell` uses for a
   * missing core reading).
   *
   * No copy lives in this component (see rule 2) — every piece of the
   * per-instant/per-range readout text is built by a host-supplied
   * callback, not a hardcoded string: `valueLabel` formats one track's
   * value at one sample (including its own "no data" case — same
   * pattern as `CpuTopologyMap`'s `tooltipLabel`), `timeLabel` formats
   * a sample index as a time label, and `rangeSummaryLabel` formats a
   * committed/previewed range into one sentence. `cursorLabel` and
   * `resetRangeLabel` are the two remaining plain-string props.
   *
   * Interaction this component owns itself (view-only, not domain
   * data, same category as `Select`'s open/closed state), and the
   * accessibility model behind it:
   * - Hovering anywhere over the plot draws one shared vertical cursor
   *   across every track and a per-track value readout (`valueLabel`
   *   read out per track) — this is a plain, always-rendered text
   *   block, so it already doubles as the readout's textual
   *   alternative for anyone not looking at the chart.
   * - The slim strip below the chart (`.brush`) is BOTH the
   *   keyboard-operable cursor control AND the range-selection
   *   control — one real `role="slider"` widget, not a decorative
   *   `tabindex` with no keyboard behavior behind it. Tab to it, then:
   *     - ArrowLeft/ArrowRight move the shared cursor one sample at a
   *       time (same `hoverIndex` a mouse hover sets).
   *     - Home/End jump the cursor to the first/last sample.
   *     - Enter/Space arms range selection at the current cursor
   *       position; moving the cursor again previews the range from
   *       that anchor to the new position (same visual preview a
   *       mouse drag shows); Enter/Space again commits it (fires
   *       `onRangeSelect`, same as releasing a mouse drag).
   *     - Escape cancels an in-progress (armed but not yet committed)
   *       keyboard selection without touching any previously
   *       committed range.
   *   `aria-valuenow`/`aria-valuetext` update on every one of those
   *   moves, so a screen reader tracks the cursor (and, while a
   *   selection is armed, the live range) the same way it would any
   *   other slider — this is why it keeps `role="slider"` instead of
   *   a plain `tabindex` div with no matching semantics.
   * - Clicking a legend swatch shows/hides that track's line (already
   *   a real `<button>`, keyboard-operable with no extra work).
   * - The committed range also renders as a visible one-line summary
   *   (`rangeSummaryLabel`) AND a visually-hidden `<table>` (real
   *   `<table>`/`<th>`/`<td>` markup, not styled-to-look-like-a-table
   *   divs) giving every visible track's value at both range
   *   endpoints — the "tabular alternative" for the selected window,
   *   reachable by a screen reader even though sighted users only see
   *   the one-line summary.
   * - The `<svg>` itself is `role="group"`, not `role="img"` — it
   *   contains real focusable children (the event-band rects, each
   *   its own `role="button"`), and `role="img"` would tell assistive
   *   tech to treat the whole thing as one flat image and hide those
   *   children from the accessibility tree, which contradicts them
   *   actually being keyboard-operable buttons.
   *
   * What it does NOT decide: what counts as an event, what a
   * selection's evidence says, or when data is stale/partial — all of
   * that is `AnalysisScreen`'s (or the host's) job, fed back in as
   * props. This component only reports the person's intent outward
   * via `onSelectEvent`/`onRangeSelect`.
   */
  import { TONE_TOKENS, type Tone } from '../tokens/tokens';

  export type AnalysisTrackKind = 'temperature' | 'clock' | 'load' | 'power';
  /**
   * `platform` = limited by the device (manufacturer thermal management
   * lowering the power limit, or an external PROCHOT) — drawn with a
   * warm striped pattern, never the thermal red. `info` = an
   * informative marker that is NOT a limitation (e.g. the end of the
   * power-turbo window): drawn as a thin dashed vertical line, never a
   * band, so it cannot be read as a problem.
   */
  export type AnalysisEventKind = 'thermal' | 'electrical' | 'mixed' | 'platform' | 'info';

  export interface AnalysisPoint {
    t: number;
    value: number | null;
    reducedQuality?: boolean;
  }

  export interface AnalysisTrack {
    kind: AnalysisTrackKind;
    label: string;
    unit?: string;
    tone: Tone;
    points: AnalysisPoint[];
    min: number;
    max: number;
  }

  export interface AnalysisEvent {
    id: string;
    kind: AnalysisEventKind;
    startT: number;
    endT: number;
    label: string;
  }

  interface Props {
    tracks: AnalysisTrack[];
    events?: AnalysisEvent[];
    selectedEventId?: string;
    onSelectEvent?: (id: string | undefined) => void;
    /** Formats one track's value at one sample — including ITS OWN "no data"/"reduced quality" wording, same pattern as CpuTopologyMap's tooltipLabel. Called with `point: undefined` when this track has no sample at that instant. */
    valueLabel: (point: AnalysisPoint | undefined, track: AnalysisTrack) => string;
    timeLabel: (t: number) => string;
    legendLabel: string;
    /** Accessible name for the keyboard-operable cursor/range-selection slider (`.brush`) — distinct from `legendLabel`, which names the track-visibility legend. */
    cursorLabel: string;
    /** One-line summary of a committed or in-progress range, e.g. `(range) => \`Muestras ${range[0]}–${range[1]}\``. Also used as the hidden range table's caption. */
    rangeSummaryLabel: (range: [number, number]) => string;
    resetRangeLabel: string;
    trackHeight?: number;
    onRangeSelect?: (range: [number, number] | null) => void;
  }

  let {
    tracks,
    events = [],
    selectedEventId,
    onSelectEvent,
    valueLabel,
    timeLabel,
    legendLabel,
    cursorLabel,
    rangeSummaryLabel,
    resetRangeLabel,
    trackHeight = 64,
    onRangeSelect
  }: Props = $props();

  const VIEW_W = 960;
  const TRACK_GAP = 6;

  let hiddenKinds = $state(new Set<AnalysisTrackKind>());
  let hoverIndex: number | null = $state(null);
  let plotEl: SVGSVGElement | undefined = $state();
  let brushEl: HTMLDivElement | undefined = $state();
  let dragStartIndex: number | null = $state(null);
  let dragCurrentIndex: number | null = $state(null);
  let rangeAnchorIndex: number | null = $state(null);
  let committedRange: [number, number] | null = $state(null);

  let maxLen = $derived(Math.max(1, ...tracks.map((t) => t.points.length)));
  let totalHeight = $derived(tracks.length * trackHeight + Math.max(0, tracks.length - 1) * TRACK_GAP);

  function indexToX(i: number): number {
    return (i / Math.max(1, maxLen - 1)) * VIEW_W;
  }
  function xToIndex(x: number): number {
    return Math.round((x / VIEW_W) * Math.max(1, maxLen - 1));
  }
  function trackTop(trackIdx: number): number {
    return trackIdx * (trackHeight + TRACK_GAP);
  }
  function valueToY(track: AnalysisTrack, trackIdx: number, value: number): number {
    const span = track.max - track.min || 1;
    const ratio = Math.max(0, Math.min(1, (value - track.min) / span));
    return trackTop(trackIdx) + (1 - ratio) * trackHeight;
  }

  interface Segment {
    d: string;
    reducedQuality: boolean;
  }

  function buildSegments(track: AnalysisTrack, trackIdx: number): Segment[] {
    const segments: Segment[] = [];
    let current: { x: number; y: number }[] = [];
    let currentRQ = false;

    function flush() {
      if (current.length > 1) {
        const d = current.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ');
        segments.push({ d, reducedQuality: currentRQ });
      }
      current = [];
    }

    for (const p of track.points) {
      if (p.value === null) {
        flush();
        continue;
      }
      const rq = !!p.reducedQuality;
      if (current.length > 0 && rq !== currentRQ) {
        // keep continuity: end previous segment at this point, start new one from it
        current.push({ x: indexToX(p.t), y: valueToY(track, trackIdx, p.value) });
        flush();
        currentRQ = rq;
        current.push({ x: indexToX(p.t), y: valueToY(track, trackIdx, p.value) });
        continue;
      }
      currentRQ = rq;
      current.push({ x: indexToX(p.t), y: valueToY(track, trackIdx, p.value) });
    }
    flush();
    return segments;
  }

  let visibleTracks = $derived(tracks.filter((t) => !hiddenKinds.has(t.kind)));
  let trackSegments = $derived(
    visibleTracks.map((t) => ({ track: t, index: tracks.indexOf(t), segments: buildSegments(t, tracks.indexOf(t)) }))
  );

  function toggleTrack(kind: AnalysisTrackKind) {
    const next = new Set(hiddenKinds);
    if (next.has(kind)) next.delete(kind);
    else next.add(kind);
    hiddenKinds = next;
  }

  function onPlotPointerMove(e: PointerEvent) {
    if (!plotEl) return;
    const rect = plotEl.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    hoverIndex = xToIndex(ratio * VIEW_W);
  }
  function onPlotPointerLeave() {
    hoverIndex = null;
  }

  function eventBandTone(kind: AnalysisEventKind): { color1: string; color2?: string; striped?: boolean } {
    if (kind === 'thermal') return { color1: 'var(--status-thermal)' };
    if (kind === 'electrical') return { color1: 'var(--status-power)' };
    if (kind === 'platform') return { color1: 'var(--status-warm)', striped: true };
    if (kind === 'info') return { color1: 'var(--text-tertiary)' };
    return { color1: 'var(--status-thermal)', color2: 'var(--status-power)' };
  }

  function indexFromClientX(el: HTMLElement, clientX: number): number {
    const rect = el.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    return Math.round(ratio * Math.max(1, maxLen - 1));
  }

  function onBrushPointerDown(e: PointerEvent) {
    if (!brushEl) return;
    (e.target as Element).setPointerCapture?.(e.pointerId);
    const i = indexFromClientX(brushEl, e.clientX);
    dragStartIndex = i;
    dragCurrentIndex = i;
  }
  function onBrushPointerMove(e: PointerEvent) {
    if (dragStartIndex === null || !brushEl) return;
    dragCurrentIndex = indexFromClientX(brushEl, e.clientX);
  }
  function onBrushPointerUp() {
    if (dragStartIndex === null || dragCurrentIndex === null) return;
    const lo = Math.min(dragStartIndex, dragCurrentIndex);
    const hi = Math.max(dragStartIndex, dragCurrentIndex);
    dragStartIndex = null;
    dragCurrentIndex = null;
    if (hi - lo < 1) {
      committedRange = null;
      onRangeSelect?.(null);
      return;
    }
    committedRange = [lo, hi];
    onRangeSelect?.([lo, hi]);
  }
  function clearRange() {
    committedRange = null;
    rangeAnchorIndex = null;
    onRangeSelect?.(null);
  }

  function onBrushFocus() {
    if (hoverIndex === null) hoverIndex = 0;
  }

  function onBrushKeydown(e: KeyboardEvent) {
    const max = maxLen - 1;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      hoverIndex = Math.max(0, (hoverIndex ?? 0) - 1);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      hoverIndex = Math.min(max, (hoverIndex ?? 0) + 1);
    } else if (e.key === 'Home') {
      e.preventDefault();
      hoverIndex = 0;
    } else if (e.key === 'End') {
      e.preventDefault();
      hoverIndex = max;
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (rangeAnchorIndex === null) {
        rangeAnchorIndex = hoverIndex ?? 0;
      } else {
        const cur = hoverIndex ?? rangeAnchorIndex;
        const lo = Math.min(rangeAnchorIndex, cur);
        const hi = Math.max(rangeAnchorIndex, cur);
        rangeAnchorIndex = null;
        if (hi - lo < 1) {
          committedRange = null;
          onRangeSelect?.(null);
        } else {
          committedRange = [lo, hi];
          onRangeSelect?.([lo, hi]);
        }
      }
    } else if (e.key === 'Escape' && rangeAnchorIndex !== null) {
      e.preventDefault();
      rangeAnchorIndex = null;
    }
  }

  let liveRangePx = $derived.by((): [number, number] | null => {
    if (dragStartIndex === null || dragCurrentIndex === null) return null;
    return [Math.min(dragStartIndex, dragCurrentIndex), Math.max(dragStartIndex, dragCurrentIndex)];
  });
  let keyboardRangePreview = $derived.by((): [number, number] | null => {
    if (rangeAnchorIndex === null) return null;
    const cur = hoverIndex ?? rangeAnchorIndex;
    return [Math.min(rangeAnchorIndex, cur), Math.max(rangeAnchorIndex, cur)];
  });
  let shownRange = $derived(liveRangePx ?? keyboardRangePreview ?? committedRange);
  let brushValueText = $derived(
    rangeAnchorIndex !== null && shownRange ? rangeSummaryLabel(shownRange) : timeLabel(hoverIndex ?? 0)
  );
</script>

<div class="tw-analysis-chart">
  <div class="legend" role="group" aria-label={legendLabel}>
    {#each tracks as track (track.kind)}
      <button
        type="button"
        class="legend-item caption"
        class:hidden={hiddenKinds.has(track.kind)}
        onclick={() => toggleTrack(track.kind)}
        aria-pressed={!hiddenKinds.has(track.kind)}
      >
        <span class="swatch" style:background={`var(--${TONE_TOKENS[track.tone]})`}></span>
        {track.label}
      </button>
    {/each}
  </div>

  <svg
    bind:this={plotEl}
    class="plot"
    viewBox={`0 0 ${VIEW_W} ${totalHeight}`}
    preserveAspectRatio="none"
    onpointermove={onPlotPointerMove}
    onpointerleave={onPlotPointerLeave}
    role="group"
    aria-label={legendLabel}
  >
    {#each events as evt (evt.id)}
      {@const tone = eventBandTone(evt.kind)}
      {@const x0 = indexToX(evt.startT)}
      {@const x1 = indexToX(evt.endT)}
      {#if evt.kind === 'info'}
        <line
          class="event-marker"
          x1={x0}
          x2={x0}
          y1={0}
          y2={totalHeight}
          stroke={tone.color1}
          stroke-width={evt.id === selectedEventId ? 2 : 1.25}
          stroke-dasharray="4 4"
          vector-effect="non-scaling-stroke"
        />
        <rect
          class="event-band marker-hit"
          x={x0 - 4}
          y={0}
          width={8}
          height={totalHeight}
          fill="transparent"
          role="button"
          tabindex="0"
          aria-label={evt.label}
          onclick={() => onSelectEvent?.(evt.id === selectedEventId ? undefined : evt.id)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') onSelectEvent?.(evt.id === selectedEventId ? undefined : evt.id);
          }}
        />
      {:else}
      <rect
        class="event-band"
        class:selected={evt.id === selectedEventId}
        x={x0}
        y={0}
        width={Math.max(2, x1 - x0)}
        height={totalHeight}
        fill={tone.color2 ? `url(#tw-mixed-${evt.id})` : tone.striped ? `url(#tw-striped-${evt.id})` : tone.color1}
        opacity={evt.id === selectedEventId ? 0.28 : 0.14}
        role="button"
        tabindex="0"
        aria-label={evt.label}
        onclick={() => onSelectEvent?.(evt.id === selectedEventId ? undefined : evt.id)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') onSelectEvent?.(evt.id === selectedEventId ? undefined : evt.id);
        }}
      />
      {#if tone.color2}
        <defs>
          <pattern id={`tw-mixed-${evt.id}`} width="10" height="10" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
            <rect width="5" height="10" fill={tone.color1} />
            <rect x="5" width="5" height="10" fill={tone.color2} />
          </pattern>
        </defs>
      {:else if tone.striped}
        <defs>
          <!-- Single-colour horizontal stripes: a pattern, not just a colour, so "limited by the device" never reads as thermal red. -->
          <pattern id={`tw-striped-${evt.id}`} width="8" height="8" patternUnits="userSpaceOnUse">
            <rect width="8" height="4" fill={tone.color1} />
          </pattern>
        </defs>
      {/if}
      <rect
        class="event-border"
        x={x0}
        y={0}
        width={Math.max(2, x1 - x0)}
        height={totalHeight}
        fill="none"
        stroke={evt.id === selectedEventId ? tone.color1 : 'transparent'}
        stroke-width="1.5"
      />
      {/if}
    {/each}

    {#if shownRange}
      <rect
        class="range-overlay"
        x={indexToX(shownRange[0])}
        y={0}
        width={Math.max(1, indexToX(shownRange[1]) - indexToX(shownRange[0]))}
        height={totalHeight}
      />
    {/if}

    {#each trackSegments as { track, index, segments } (track.kind)}
      <g>
        <rect
          x="0"
          y={trackTop(index)}
          width={VIEW_W}
          height={trackHeight}
          fill="var(--surface-sunken)"
          opacity="0.4"
        />
        {#each segments as seg, i (i)}
          <path
            d={seg.d}
            fill="none"
            stroke={`var(--${TONE_TOKENS[track.tone]})`}
            stroke-width="1.8"
            stroke-linejoin="round"
            stroke-linecap="round"
            stroke-dasharray={seg.reducedQuality ? '3 3' : undefined}
          />
        {/each}
      </g>
    {/each}

    {#if hoverIndex !== null}
      <line class="cursor" x1={indexToX(hoverIndex)} x2={indexToX(hoverIndex)} y1="0" y2={totalHeight} />
    {/if}
  </svg>

  {#if hoverIndex !== null}
    <div class="readout caption">
      <span class="readout-time">{timeLabel(hoverIndex)}</span>
      {#each tracks as track (track.kind)}
        {#if !hiddenKinds.has(track.kind)}
          {@const p = track.points.find((pt) => pt.t === hoverIndex)}
          <span class="readout-item">
            <span class="swatch" style:background={`var(--${TONE_TOKENS[track.tone]})`}></span>
            {valueLabel(p, track)}
          </span>
        {/if}
      {/each}
    </div>
  {/if}

  <div
    class="brush"
    bind:this={brushEl}
    onpointerdown={onBrushPointerDown}
    onpointermove={onBrushPointerMove}
    onpointerup={onBrushPointerUp}
    onkeydown={onBrushKeydown}
    onfocus={onBrushFocus}
    role="slider"
    aria-label={cursorLabel}
    aria-valuemin={0}
    aria-valuemax={maxLen - 1}
    aria-valuenow={hoverIndex ?? 0}
    aria-valuetext={brushValueText}
    tabindex="0"
  >
    <div class="brush-track"></div>
    {#if shownRange}
      <div
        class="brush-selection"
        style:left={`${(shownRange[0] / Math.max(1, maxLen - 1)) * 100}%`}
        style:width={`${((shownRange[1] - shownRange[0]) / Math.max(1, maxLen - 1)) * 100}%`}
      ></div>
    {/if}
  </div>

  {#if shownRange}
    <p class="range-summary body">{rangeSummaryLabel(shownRange)}</p>
    <table class="sr-only">
      <caption>{rangeSummaryLabel(shownRange)}</caption>
      <thead>
        <tr>
          <th scope="col">{legendLabel}</th>
          <th scope="col">{timeLabel(shownRange[0])}</th>
          <th scope="col">{timeLabel(shownRange[1])}</th>
        </tr>
      </thead>
      <tbody>
        {#each tracks as track (track.kind)}
          <tr>
            <th scope="row">{track.label}</th>
            <td>{valueLabel(track.points.find((p) => p.t === shownRange[0]), track)}</td>
            <td>{valueLabel(track.points.find((p) => p.t === shownRange[1]), track)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if committedRange}
    <button type="button" class="clear-range caption" onclick={clearRange}>{resetRangeLabel}</button>
  {/if}
</div>

<style>
  @keyframes tw-band-in {
    from {
      opacity: 0;
      transform: scaleY(0.2);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .tw-analysis-chart {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--hairline);
    background: var(--surface);
    color: var(--text-primary);
    padding: 4px 10px;
    border-radius: var(--radius-full);
  }
  .legend-item.hidden {
    color: var(--text-tertiary);
    opacity: 0.55;
  }
  .legend-item:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .swatch {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: 0 0 auto;
  }
  .plot {
    width: 100%;
    height: auto;
    display: block;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: crosshair;
  }
  .event-band {
    transform-box: fill-box;
    transform-origin: center;
    animation: tw-band-in var(--motion-slow) var(--motion-ease-out) both;
    cursor: pointer;
  }
  .event-band:focus-visible {
    outline: 2px solid var(--accent-blue);
  }
  .range-overlay {
    fill: color-mix(in srgb, var(--accent-blue) 10%, transparent);
    pointer-events: none;
  }
  .cursor {
    stroke: var(--text-tertiary);
    stroke-width: 1;
    pointer-events: none;
  }
  .readout {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: center;
    color: var(--text-secondary);
    background: var(--surface-raised);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
  }
  .readout-time {
    color: var(--text-primary);
    font-weight: 650;
  }
  .readout-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .brush {
    position: relative;
    height: 18px;
    display: flex;
    align-items: center;
    touch-action: none;
  }
  .brush-track {
    position: absolute;
    inset: 7px 0;
    background: var(--surface-sunken);
    border-radius: var(--radius-full);
  }
  .brush-selection {
    position: absolute;
    top: 2px;
    bottom: 2px;
    background: color-mix(in srgb, var(--accent-blue) 35%, transparent);
    border: 1px solid var(--accent-blue);
    border-radius: var(--radius-sm);
  }
  .brush:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }
  .range-summary {
    margin: 0;
    color: var(--text-secondary);
  }
  .clear-range {
    align-self: flex-start;
    border: none;
    background: transparent;
    color: var(--accent-blue);
    padding: 2px 4px;
  }
  /* Visually hidden but reachable by assistive tech — the tabular
     alternative for the currently shown range (see doc comment). */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
