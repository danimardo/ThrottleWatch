<script lang="ts">
  /**
   * A determinate (or indeterminate) linear progress track. Extracted
   * out of StatusHero's own performance-bar markup so every progress
   * indicator in the app — the performance-headroom bar, an
   * update-download bar, a scan/diagnóstico-guiado-in-progress bar —
   * is the same 4px hairline-bounded track instead of each screen
   * reinventing it slightly differently.
   *
   * `tone` reuses the same Tone union as StatWidget/CausalRail
   * (accent/thermal/warm/normal/power/unknown) rather than inventing a
   * separate color vocabulary for progress — a download uses
   * `accent`, a bar tied to a classification's severity uses whichever
   * status tone that classification maps to.
   *
   * Pass `indeterminate` for "in progress, no known percentage yet"
   * (e.g. before a diagnóstico guiado step has produced a first
   * reading) — never fake a percentage to get a determinate bar to
   * render instead.
   */
  import { TONE_TOKENS, type Tone } from '../tokens/tokens';

  interface Props {
    /** 0–100. Ignored when `indeterminate` is set. */
    percent?: number;
    tone?: Tone;
    indeterminate?: boolean;
    /** Accessible label, e.g. "Descargando actualización" — read by aria-label. */
    label?: string;
  }

  let { percent = 0, tone = 'accent', indeterminate = false, label }: Props = $props();

  let clamped = $derived(Math.max(0, Math.min(100, percent)));
  let colorVar = $derived(`var(--${TONE_TOKENS[tone]})`);
</script>

<div
  class="tw-progress"
  role="progressbar"
  aria-label={label}
  aria-valuenow={indeterminate ? undefined : clamped}
  aria-valuemin={0}
  aria-valuemax={100}
>
  {#if indeterminate}
    <div class="fill indeterminate" style:background={colorVar}></div>
  {:else}
    <div class="fill" style:width={`${clamped}%`} style:background={colorVar}></div>
  {/if}
</div>

<style>
  .tw-progress {
    height: 4px;
    width: 100%;
    border-radius: var(--radius-full);
    background: var(--surface-sunken);
    position: relative;
    overflow: hidden;
  }
  .fill {
    position: absolute;
    inset: 0;
    border-radius: var(--radius-full);
  }
  .fill:not(.indeterminate) {
    left: 0;
    right: auto;
    transition: width var(--motion-slow) var(--motion-spring);
    overflow: hidden;
  }
  /* One specular sweep along the fill each time it changes size. */
  .fill:not(.indeterminate)::after {
    content: '';
    position: absolute;
    inset: 0;
    width: 40%;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.55), transparent);
    animation: tw-sheen-sweep 1.1s var(--motion-ease-in-out) 1;
  }
  .fill.indeterminate {
    width: 40%;
    animation: tw-progress-sweep 1.3s ease-in-out infinite;
  }
  @keyframes tw-progress-sweep {
    0% {
      left: -40%;
    }
    100% {
      left: 100%;
    }
  }
</style>
