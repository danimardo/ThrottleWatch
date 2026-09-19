<script lang="ts">
  /**
   * One pill for a value of diagnostic_report.classification. Color
   * and icon come from lib/classification.ts — CLASSIFICATION_META —
   * never chosen ad hoc here, so a classification can't drift to a
   * different color in one screen than another.
   *
   * `label` is the already-translated string to display (this system
   * ships no copy — the app's i18n catalog owns the seven strings).
   *
   * Never place an `indeterminate` chip next to a `normal` chip in the
   * same context without distinguishing which is the active state —
   * they are semantic opposites (see lib/classification.ts).
   */
  import StatusIcon from '../icons/StatusIcon.svelte';
  import { CLASSIFICATION_META, type Classification } from '../lib/classification';

  interface Props {
    classification: Classification;
    label: string;
  }

  let { classification, label }: Props = $props();
  let meta = $derived(CLASSIFICATION_META[classification]);
</script>

<div
  class="chip tag"
  style:color={meta.neutralBg ? 'var(--text-primary)' : `var(--${meta.token})`}
  style:background={meta.neutralBg ? 'var(--surface-raised)' : `color-mix(in srgb, var(--${meta.token}) ${meta.bgOpacity * 100}%, transparent)`}
>
  <span class="icon"><StatusIcon kind={meta.icon} /></span>
  {label}
</div>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-full);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    width: fit-content;
  }
  .icon {
    width: 11px;
    height: 11px;
    display: flex;
    flex: 0 0 auto;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
