<script lang="ts">
  /**
   * The dot row under each onboarding slide's copy — done steps filled,
   * the current step filled and wider, remaining steps outline-only.
   * Deliberately not a percentage bar: 5 discrete steps read better as
   * discrete dots than as a continuous ProgressBar, and a dot row
   * doubles as a lightweight "how many are left" cue the ProgressBar's
   * shape doesn't give you.
   *
   * `label` is the accessible name for the whole indicator (something
   * like "Paso 2 de 5" — translated by the consumer; this component has
   * no copy of its own). The numeric fraction rendered inside
   * `aria-valuetext` falls back to plain digits (`"2/5"`) when you don't
   * override it, so the indicator is never silently unlabeled even
   * before you wire real copy.
   */
  interface Props {
    stepCount: number;
    /** 0-based index of the current step. */
    currentStep: number;
    label?: string;
  }

  let { stepCount, currentStep, label }: Props = $props();
</script>

<div
  class="tw-onboarding-progress"
  role="progressbar"
  aria-valuemin={1}
  aria-valuemax={stepCount}
  aria-valuenow={currentStep + 1}
  aria-valuetext={label ?? `${currentStep + 1}/${stepCount}`}
  aria-label={label}
>
  {#each { length: stepCount } as _, i (i)}
    <span
      class="dot"
      class:done={i < currentStep}
      class:current={i === currentStep}
    ></span>
  {/each}
</div>

<style>
  .tw-onboarding-progress {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--surface-sunken);
    border: 1px solid var(--hairline);
    transition: width 0.16s ease, background 0.16s ease;
  }
  .dot.done {
    background: var(--accent-blue);
    border-color: transparent;
  }
  .dot.current {
    width: 18px;
    background: var(--accent-blue);
    border-color: transparent;
  }
</style>
