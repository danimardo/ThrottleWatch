<script lang="ts">
  /**
   * The "Informe" screen — a diagnosis report told as a narrative, not
   * a dashboard: resultado en una frase, qué se observó, impacto
   * estimado, evidencias, causas alternativas, qué no puede
   * concluirse, recomendaciones, método del impacto (its inputs:
   * power limit and measured power, or the guided test result), and a
   * before/after comparison between two comparable guided tests. Presentational, like every screen in
   * this system — every section's copy and every stat is
   * host-supplied; this component never computes an impact
   * percentage, never decides which causes are "alternative," and
   * never invents a caveat of its own.
   *
   * A section is omitted entirely when its content prop is empty or
   * undefined — the host passes `undefined`/`[]` rather than this
   * component padding an empty heading over nothing (an "Evidencias"
   * heading with no evidence reads worse than no heading at all).
   *
   * `causalChain` maps straight to `CausalRail` and follows THAT
   * component's own rule: pass it only when there is a real 2–4 node
   * evidence chain backed by the diagnosis, leave it undefined
   * otherwise — never invented here just to fill the "qué se
   * observó" section.
   *
   * Three notices are independent booleans, not a single enum,
   * because a session can be incomplete AND running on reduced
   * confidence AND lacking the inputs of a method all at once. They stack in a
   * fixed order (incomplete, then reduced confidence) above the
   * narrative so simultaneous notices always read the same way; the
   * missing-method case has no banner of its own — it renders
   * inline as the method section's own missing-state text, since it isn't a warning about this report so much as a
   * plain statement of what evidence the comparison rests on.
   *
   * `impactValue` absent means "sin cifra: faltan las entradas del
   * método (límite de potencia) o la confianza es insuficiente"; when
   * present it is a cooling-potential band/range ("+10–20 %") or a
   * measured guided result, never a clock ratio — the host decides why (via
   * `impactUnavailableReason`), this component just shows whichever
   * of the two states it's given.
   *
   * `comparisonBeforeLabel`/`comparisonAfterLabel` are the before/after
   * table's own two column headers — previously hardcoded "Antes"/
   * "Después" text, now props like everything else here (rule 2).
   */
  import type { Snippet } from 'svelte';
  import StatusChip from './StatusChip.svelte';
  import Banner from './Banner.svelte';
  import CausalRail, { type CausalNode } from './CausalRail.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import type { Classification } from '../lib/classification';

  export interface ReportComparisonMetric {
    label: string;
    beforeValue: string;
    afterValue: string;
  }

  interface Props {
    status: 'loading' | 'ready';
    loadingLabel: string;

    classification: Classification;
    classificationLabel: string;
    headline: string;

    sessionIncomplete?: boolean;
    incompleteNoticeTitle?: string;
    incompleteNoticeDescription?: string;

    /** Session still running: the report is a live, non-persisted evaluation. */
    provisional?: boolean;
    provisionalNoticeTitle?: string;
    provisionalNoticeDescription?: string;
    /** Imported session re-evaluated with the current ruleset (FR-073): shown next to, never instead of, the original. */
    reevaluated?: {
      title: string;
      classification: Classification;
      classificationLabel: string;
      headline: string;
      note?: string;
    };
    reducedConfidence?: boolean;
    reducedConfidenceNoticeTitle?: string;
    reducedConfidenceNoticeDescription?: string;

    observedTitle: string;
    observedText: string;
    causalChain?: CausalNode[];

    impactTitle: string;
    impactValue?: string;
    impactUnavailableTitle?: string;
    impactUnavailableReason?: string;

    evidenceTitle: string;
    evidence?: string[];

    alternativeCausesTitle: string;
    alternativeCauses?: string[];

    cannotConcludeTitle: string;
    cannotConclude?: string[];

    recommendationsTitle: string;
    recommendations?: string[];

    methodTitle: string;
    methodDescription?: string;
    methodMissingText?: string;

    comparisonTitle: string;
    comparisonBeforeLabel?: string;
    comparisonAfterLabel?: string;
    comparisonMetrics?: ReportComparisonMetric[];

    exportActions?: Snippet;
  }

  let {
    status,
    loadingLabel,
    classification,
    classificationLabel,
    headline,
    sessionIncomplete = false,
    incompleteNoticeTitle,
    incompleteNoticeDescription,
    provisional = false,
    provisionalNoticeTitle,
    provisionalNoticeDescription,
    reevaluated,
    reducedConfidence = false,
    reducedConfidenceNoticeTitle,
    reducedConfidenceNoticeDescription,
    observedTitle,
    observedText,
    causalChain,
    impactTitle,
    impactValue,
    impactUnavailableTitle,
    impactUnavailableReason,
    evidenceTitle,
    evidence = [],
    alternativeCausesTitle,
    alternativeCauses = [],
    cannotConcludeTitle,
    cannotConclude = [],
    recommendationsTitle,
    recommendations = [],
    methodTitle,
    methodDescription,
    methodMissingText,
    comparisonTitle,
    comparisonBeforeLabel,
    comparisonAfterLabel,
    comparisonMetrics = [],
    exportActions
  }: Props = $props();
</script>

<div class="tw-ds tw-report-screen">
  {#if status === 'loading'}
    <div class="status-block">
      <ProgressBar indeterminate tone="accent" label={loadingLabel} />
      <span class="caption" style:color="var(--text-tertiary)">{loadingLabel}</span>
    </div>
  {:else}
    <header class="header">
      <div class="header-top">
        <StatusChip {classification} label={classificationLabel} />
        {#if exportActions}
          <div class="export-actions">{@render exportActions()}</div>
        {/if}
      </div>
      <h2 class="value-lg headline" style:color="var(--text-primary)">{headline}</h2>
    </header>

    {#if provisional && provisionalNoticeTitle}
      <Banner tone="info" title={provisionalNoticeTitle} description={provisionalNoticeDescription} />
    {/if}
    {#if reevaluated}
      <section class="block reevaluated" style:--tw-i=0>
        <span class="caption section-label" style:color="var(--text-tertiary)">{reevaluated.title}</span>
        <div class="reeval-row">
          <StatusChip classification={reevaluated.classification} label={reevaluated.classificationLabel} />
          <span class="body-strong" style:color="var(--text-primary)">{reevaluated.headline}</span>
        </div>
        {#if reevaluated.note}
          <span class="caption" style:color="var(--text-tertiary)">{reevaluated.note}</span>
        {/if}
      </section>
    {/if}
    {#if sessionIncomplete && incompleteNoticeTitle}
      <Banner tone="warning" title={incompleteNoticeTitle} description={incompleteNoticeDescription} />
    {/if}
    {#if reducedConfidence && reducedConfidenceNoticeTitle}
      <Banner tone="warning" title={reducedConfidenceNoticeTitle} description={reducedConfidenceNoticeDescription} />
    {/if}

    <section class="block" style:--tw-i=1>
      <span class="caption section-label" style:color="var(--text-tertiary)">{observedTitle}</span>
      <p class="body block-text" style:color="var(--text-secondary)">{observedText}</p>
      {#if causalChain && causalChain.length >= 2}
        <div class="causal-wrap"><CausalRail nodes={causalChain} /></div>
      {/if}
    </section>

    <section class="block" style:--tw-i=2>
      <span class="caption section-label" style:color="var(--text-tertiary)">{impactTitle}</span>
      {#if impactValue}
        <span class="value-lg impact-value" style:color="var(--text-primary)">{impactValue}</span>
      {:else}
        <div class="impact-unavailable">
          {#if impactUnavailableTitle}
            <span class="body-strong" style:color="var(--text-primary)">{impactUnavailableTitle}</span>
          {/if}
          {#if impactUnavailableReason}
            <span class="body" style:color="var(--text-tertiary)">{impactUnavailableReason}</span>
          {/if}
        </div>
      {/if}
    </section>

    {#if evidence.length > 0}
      <section class="block" style:--tw-i=3>
        <span class="caption section-label" style:color="var(--text-tertiary)">{evidenceTitle}</span>
        <ul class="body list">
          {#each evidence as item, i (i)}
            <li style:color="var(--text-secondary)">{item}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if comparisonMetrics.length > 0}
      <section class="block" style:--tw-i=4>
        <span class="caption section-label" style:color="var(--text-tertiary)">{comparisonTitle}</span>
        <div class="comparison-table" role="table">
          <div class="comparison-row comparison-head" role="row">
            <span class="caption" role="columnheader" style:color="var(--text-tertiary)"></span>
            <span class="caption" role="columnheader" style:color="var(--text-tertiary)">{comparisonBeforeLabel ?? ''}</span>
            <span class="caption" role="columnheader" style:color="var(--text-tertiary)">{comparisonAfterLabel ?? ''}</span>
          </div>
          {#each comparisonMetrics as metric (metric.label)}
            <div class="comparison-row" role="row">
              <span class="body" role="cell" style:color="var(--text-secondary)">{metric.label}</span>
              <span class="body-strong" role="cell" style:color="var(--text-primary)">{metric.beforeValue}</span>
              <span class="body-strong" role="cell" style:color="var(--text-primary)">{metric.afterValue}</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if alternativeCauses.length > 0}
      <section class="block" style:--tw-i=5>
        <span class="caption section-label" style:color="var(--text-tertiary)">{alternativeCausesTitle}</span>
        <ul class="body list">
          {#each alternativeCauses as item, i (i)}
            <li style:color="var(--text-secondary)">{item}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if cannotConclude.length > 0}
      <section class="block" style:--tw-i=6>
        <span class="caption section-label" style:color="var(--text-tertiary)">{cannotConcludeTitle}</span>
        <ul class="body list">
          {#each cannotConclude as item, i (i)}
            <li style:color="var(--text-secondary)">{item}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if recommendations.length > 0}
      <section class="block" style:--tw-i=7>
        <span class="caption section-label" style:color="var(--text-tertiary)">{recommendationsTitle}</span>
        <ul class="body list numbered">
          {#each recommendations as item, i (i)}
            <li style:color="var(--text-secondary)">{item}</li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="block method-block" style:--tw-i=8>
      <span class="caption section-label" style:color="var(--text-tertiary)">{methodTitle}</span>
      {#if methodDescription}
        <p class="body block-text" style:color="var(--text-secondary)">{methodDescription}</p>
      {:else}
        <p class="body block-text" style:color="var(--text-tertiary)">{methodMissingText}</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .reevaluated {
    border: 1px dashed var(--hairline);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
  }
  .reeval-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .tw-report-screen {
    background: transparent;
    padding: var(--space-6);
    max-width: 720px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    container-type: inline-size;
    container-name: tw-report;
  }
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-6) 0;
  }
  .header {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .header-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .export-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .headline {
    margin: 0;
    line-height: 1.3;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    animation: tw-rise var(--motion-slow) var(--motion-ease-out) both;
    animation-delay: calc(var(--tw-i, 0) * var(--motion-stagger));
  }
  .section-label {
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .block-text {
    margin: 0;
    max-width: 62ch;
  }
  .causal-wrap {
    margin-top: var(--space-2);
  }
  .impact-value {
    display: block;
  }
  .impact-unavailable {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list {
    margin: 0;
    padding-left: 1.25em;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .list li {
    max-width: 62ch;
  }
  .list.numbered {
    list-style: decimal;
  }
  .list:not(.numbered) {
    list-style: disc;
  }
  .comparison-table {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .comparison-row {
    display: grid;
    grid-template-columns: 1fr 96px 96px;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    align-items: center;
  }
  .comparison-row:not(:last-child) {
    border-bottom: 1px solid var(--hairline);
  }
  .comparison-head {
    background: var(--surface-sunken);
  }
  .method-block {
    padding-top: var(--space-3);
    border-top: 1px solid var(--hairline);
  }

  @container tw-report (max-width: 479px) {
    .comparison-row {
      grid-template-columns: 1fr 72px 72px;
      gap: var(--space-2);
      padding: var(--space-2) var(--space-3);
    }
  }
</style>
