<script lang="ts">
  import type { StatusIconKind } from '../lib/classification';

  interface Props {
    kind: StatusIconKind;
  }

  let { kind }: Props = $props();
</script>

<!--
  The fixed icon set tied to diagnostic_report.classification (see
  lib/classification.ts). These are not consumer-swappable — the whole
  point of CLASSIFICATION_META is that a classification always renders
  with the same icon and color everywhere in the app. Line icons,
  stroke-width 2.2–2.4, round caps/joins, viewBox 0 0 24 24, sized by
  the parent (width/height: 100%) and colored with currentColor except
  "mixed", whose two strokes are always status-thermal + status-power.
-->
{#if kind === 'check'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 12l5 5L20 6" />
  </svg>
{:else if kind === 'warning'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 2 2 20h20L12 2Zm0 7v5" />
    <circle cx="12" cy="17" r="1" fill="currentColor" />
  </svg>
{:else if kind === 'flame'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M8 3c0 4-4 5-4 10a6 6 0 0 0 12 0c0-2-1-3-2-4.5 0 2-1 3-2 3-1.5 0-2-1.5-1-3.5C10 6 9 4.5 8 3Z" />
  </svg>
{:else if kind === 'bolt'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M13 2 4 14h6l-1 8 9-12h-6z" />
  </svg>
{:else if kind === 'device'}
  <!-- A laptop with a small heat wave above the lid: "the device", not the CPU. -->
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
    <rect x="5" y="9" width="14" height="8" rx="1.2" />
    <path d="M3 20h18" />
    <path d="M9 6c.6-.7.6-1.3 0-2M12 6c.6-.7.6-1.3 0-2M15 6c.6-.7.6-1.3 0-2" />
  </svg>
{:else if kind === 'unknown'}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <path d="M9.5 9a2.5 2.5 0 0 1 4.7 1.2c0 1.5-2.2 1.6-2.2 3.3" />
    <circle cx="12" cy="17" r=".6" fill="currentColor" />
  </svg>
{:else if kind === 'mixed'}
  <svg viewBox="0 0 24 24" fill="none" stroke-linecap="round" stroke-linejoin="round">
    <path
      d="M8 3c0 4-4 5-4 10a6 6 0 0 0 12 0c0-2-1-3-2-4.5 0 2-1 3-2 3-1.5 0-2-1.5-1-3.5C10 6 9 4.5 8 3Z"
      stroke="var(--status-thermal)"
      stroke-width="2.1"
    />
    <path
      d="M13 8 9 15h4l-.5 4 4.5-6h-3z"
      stroke="var(--status-power)"
      stroke-width="2.1"
      fill="none"
    />
  </svg>
{/if}
