<script lang="ts">
  /**
   * The one prominent conclusion on the "Ahora" screen — it never
   * competes with a second card of the same visual weight.
   *
   * The ring and the status tag ALWAYS share the same classification
   * color (never two different colors for the same classification
   * inside this component) — that mapping lives in
   * lib/classification.ts and is not configurable here.
   *
   * The performance block is the one place in this system where a
   * safety rule is enforced in code rather than left to the consumer's
   * discipline: if you do not pass `performance`, this component
   * renders "Aún sin referencia comparable" (or your `noBaselineText`
   * override) instead of silently omitting the block or letting
   * someone compute a number without a baseline. See the product
   * constitution: a performance percentage only exists with a valid
   * local baseline.
   *
   * Layout is self-contained: it uses a CSS container query on its own
   * width (not the window's) to switch from a row to a stacked column
   * below 900px of available width, per the system's responsive table.
   * You do not need to pass a layout prop.
   */
  import StatusIcon from '../icons/StatusIcon.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { CLASSIFICATION_META, type Classification } from '../lib/classification';

  interface PerformanceInfo {
    /** e.g. "Rendimiento disponible" / "Available performance" */
    label: string;
    /** e.g. "78–84 %" — the range as already-formatted text */
    rangeText: string;
    /** 0–100, drives the bar fill. Use the midpoint of the range, or the conservative bound. */
    percent: number;
  }

  interface Props {
    /** Big number in the ring center, e.g. "98°". */
    ringValue: string;
    /** Small caption under it, e.g. "TjMax 100°". */
    ringCaption: string;
    /** 0–100, how much of the ring's circumference is drawn. */
    ringPercent: number;
    classification: Classification;
    /** Already-translated classification label, e.g. "LIMITACIÓN TÉRMICA CONFIRMADA". */
    classificationLabel: string;
    /** The evidence/confidence line under the tag, e.g. "4 °C hasta el límite · confianza alta · observado 3 min 42 s". */
    evidenceLine: string;
    /** Omit or pass null/undefined when there is no valid local baseline. */
    performance?: PerformanceInfo | null;
    /** Override the no-baseline fallback string (defaults to the Spanish copy). */
    noBaselineText?: string;
  }

  let {
    ringValue,
    ringCaption,
    ringPercent,
    classification,
    classificationLabel,
    evidenceLine,
    performance = null,
    noBaselineText = 'Aún sin referencia comparable'
  }: Props = $props();

  let meta = $derived(CLASSIFICATION_META[classification]);

  const R = 55;
  const C = 2 * Math.PI * R;
  let dashoffset = $derived(C * (1 - Math.max(0, Math.min(100, ringPercent)) / 100));
  let ringColor = $derived(meta.neutralBg ? 'var(--text-primary)' : `var(--${meta.token})`);
</script>

<div class="hero">
  <div class="content">
    <div class="ring" style:--tw-ring-color={ringColor}>
      <!-- Soft halo behind the ring: breathes very slowly (the one
           continuous motion allowed at rest), stronger on a state
           change. Decorative only: aria-hidden, no information. -->
      <span class="halo" aria-hidden="true"></span>
      <svg viewBox="0 0 130 130">
        <circle cx="65" cy="65" r={R} fill="none" stroke="var(--surface-sunken)" stroke-width="11" />
        <circle
          cx="65"
          cy="65"
          r={R}
          fill="none"
          class="arc"
          stroke={ringColor}
          stroke-width="11"
          stroke-linecap="round"
          stroke-dasharray={C}
          stroke-dashoffset={dashoffset}
          transform="rotate(-90 65 65)"
        />
      </svg>
      <div class="ring-center">
        {#key ringValue}
          <span class="value-md value-swap">{ringValue}</span>
        {/key}
        <span class="caption" style:color="var(--text-tertiary)">{ringCaption}</span>
      </div>
    </div>

    <div class="middle">
      {#key classification}
      <span
        class="status-tag tag tw-pop"
        style:color={meta.neutralBg ? 'var(--text-primary)' : `var(--${meta.token})`}
        style:background={meta.neutralBg ? 'var(--surface-raised)' : `color-mix(in srgb, var(--${meta.token}) ${meta.bgOpacity * 100}%, transparent)`}
      >
        <span class="icon"><StatusIcon kind={meta.icon} /></span>
        {classificationLabel}
      </span>
      {/key}
      <span class="body" style:color="var(--text-secondary)">{evidenceLine}</span>
    </div>

    <div class="perf-block">
      {#if performance}
        <span class="label" style:color="var(--text-secondary)">{performance.label}</span>
        <div class="value-lg" style:color="var(--accent-blue)">{performance.rangeText}</div>
        <div class="perf-bar">
          <ProgressBar percent={performance.percent} tone="accent" label={performance.label} />
        </div>
      {:else}
        <span class="body" style:color="var(--text-tertiary)">{noBaselineText}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .hero {
    container-type: inline-size;
    container-name: tw-hero;
    width: 100%;
  }
  .content {
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    display: flex;
    align-items: center;
    gap: var(--space-6);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-lg);
    padding: var(--space-5) var(--space-6);
  }
  .ring {
    width: 130px;
    height: 130px;
    position: relative;
    flex: 0 0 auto;
  }
  .halo {
    position: absolute;
    inset: 6px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--tw-ring-color) 28%, transparent) 55%, transparent 72%);
    filter: blur(6px);
    animation: tw-breathe 6s var(--motion-ease-in-out) infinite;
    pointer-events: none;
  }
  .ring svg {
    position: relative;
  }
  .arc {
    transition:
      stroke-dashoffset var(--motion-slow) var(--motion-ease-out),
      stroke var(--motion-base) linear;
  }
  .value-swap {
    animation: tw-rise var(--motion-base) var(--motion-ease-out) both;
  }
  .ring svg {
    width: 100%;
    height: 100%;
  }
  .ring-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }
  .middle {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }
  .status-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    width: fit-content;
  }
  .status-tag .icon {
    width: 12px;
    height: 12px;
    display: flex;
  }
  .status-tag .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .perf-block {
    flex: 0 0 auto;
    width: 180px;
    padding-left: var(--space-5);
    border-left: 1px solid var(--hairline);
  }
  .perf-block .perf-bar {
    margin-top: 6px;
  }

  /* Mobile-first: stack below 900px of the component's OWN available
     width (not the window's) — ring on top, tag+evidence next,
     performance block last, no left border. */
  @container tw-hero (max-width: 899px) {
    .content {
      flex-direction: column;
      align-items: stretch;
      text-align: center;
    }
    .ring {
      align-self: center;
    }
    .middle {
      align-items: center;
      text-align: center;
    }
    .perf-block {
      width: auto;
      padding-left: 0;
      border-left: none;
      text-align: center;
    }
  }
</style>
