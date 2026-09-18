<script lang="ts">
  /**
   * The always-visible strip above StatusHero on "Ahora" (ux-visual-spec
   * → "Franja superior"): processor + topology, power source/plan,
   * collector state with sample freshness, and the "Ver cobertura"
   * action. One line at medium/expanded width, two lines in compact.
   *
   * `collectorState` drives a colored dot AND the accompanying text —
   * never color alone: `fresh` → status-normal, `stale` → status-warm,
   * `disconnected` → status-unknown, `starting` → accent-blue. The
   * host decides which state applies (e.g. "stale" after > 5 s without
   * a sample, per spec) — this component only renders it.
   *
   * No copy lives here: every string, including "hace 1 s", is a prop.
   */
  import ToolbarButton from './ToolbarButton.svelte';
  import type { Snippet } from 'svelte';

  export type CollectorState = 'fresh' | 'stale' | 'disconnected' | 'starting';

  interface Props {
    cpuLabel: string;
    /** e.g. "8P + 16E" — omitted for a homogeneous CPU when the host prefers. */
    topologyLabel?: string;
    /** e.g. "CA · Equilibrado" / "Batería 64 % · Ahorro". Omit when unknown. */
    powerLabel?: string;
    collectorState: CollectorState;
    /** e.g. "Conectado · hace 1 s" / "Datos obsoletos · hace 12 s" / "Colector desconectado". */
    collectorLabel: string;
    coverageActionLabel?: string;
    onCoverage?: () => void;
    coverageIcon?: Snippet;
    compact?: boolean;
  }

  let {
    cpuLabel,
    topologyLabel,
    powerLabel,
    collectorState,
    collectorLabel,
    coverageActionLabel,
    onCoverage,
    coverageIcon,
    compact = false
  }: Props = $props();

  const STATE_TOKEN: Record<CollectorState, string> = {
    fresh: 'status-normal',
    stale: 'status-warm',
    disconnected: 'status-unknown',
    starting: 'accent-blue'
  };

  // Reconnection glow: when the collector comes back to `fresh` from
  // any other state, the dot emits one expanding ring. Decorative.
  let reconnected = $state(false);
  let prevState: CollectorState | undefined;
  $effect(() => {
    const now = collectorState;
    const was = prevState;
    prevState = now;
    if (was !== undefined && was !== 'fresh' && now === 'fresh') {
      reconnected = true;
      const t = setTimeout(() => (reconnected = false), 900);
      return () => clearTimeout(t);
    }
  });
</script>

{#snippet defaultCoverageIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="M3 10h18M9 10v10" />
  </svg>
{/snippet}

<div class="tw-ds tw-context-strip" class:compact>
  <div class="group cpu">
    <span class="body-strong" style:color="var(--text-primary)">{cpuLabel}</span>
    {#if topologyLabel}
      <span class="caption" style:color="var(--text-tertiary)">{topologyLabel}</span>
    {/if}
  </div>

  <div class="group status">
    {#if powerLabel}
      <span class="body" style:color="var(--text-secondary)">{powerLabel}</span>
      <span class="sep" aria-hidden="true">·</span>
    {/if}
    <span class="collector" style:--tw-strip-color={`var(--${STATE_TOKEN[collectorState]})`}>
      <span class="dot" class:pulse={collectorState === 'starting'} class:glow={reconnected} aria-hidden="true"></span>
      <span class="body" style:color={collectorState === 'fresh' ? 'var(--text-secondary)' : 'var(--tw-strip-color)'}>{collectorLabel}</span>
    </span>
  </div>

  {#if onCoverage && coverageActionLabel}
    <div class="group action">
      <ToolbarButton icon={coverageIcon ?? defaultCoverageIcon} label={coverageActionLabel} {compact} onclick={onCoverage} />
    </div>
  {/if}
</div>

<style>
  .tw-context-strip {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-3);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    min-width: 0;
  }
  .group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .cpu {
    flex: 1 1 auto;
    overflow: hidden;
  }
  .cpu .body-strong {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status {
    flex: 0 1 auto;
    white-space: nowrap;
  }
  .sep {
    color: var(--text-tertiary);
  }
  .collector {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--tw-strip-color);
    display: inline-block;
  }
  .dot.pulse {
    animation: tw-strip-pulse 1.2s ease-in-out infinite;
  }
  .dot.glow {
    --tw-glow-color: var(--tw-strip-color);
    animation: tw-glow 0.9s var(--motion-ease-out) 1;
  }
  @keyframes tw-strip-pulse {
    50% {
      opacity: 0.35;
    }
  }
  .action {
    flex: 0 0 auto;
    margin-left: auto;
  }
  .tw-context-strip.compact {
    flex-wrap: wrap;
    row-gap: var(--space-2);
  }
  .tw-context-strip.compact .cpu {
    flex-basis: 100%;
  }
  .tw-context-strip.compact .status {
    flex: 1 1 auto;
    white-space: normal;
  }
</style>
