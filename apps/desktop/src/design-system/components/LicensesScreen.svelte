<script lang="ts">
  /**
   * "Licencias de terceros" (constitution quality gate 8, T111): one
   * expandable entry per dependency with name, version, license id
   * and the full notice text, served from the packaged notices
   * (`get_third_party_notices`). Expansion is view-only local state.
   *
   * Uses a native <details> so the text stays reachable by keyboard
   * and screen readers without extra ARIA; the marker is restyled to
   * this system's chevron.
   */
  export interface LicenseEntry {
    id: string;
    name: string;
    version?: string;
    /** SPDX id or human name, e.g. "MPL-2.0". */
    license: string;
    /** Full license/notice text. */
    text: string;
  }

  interface Props {
    title: string;
    intro?: string;
    entries: LicenseEntry[];
  }

  let { title, intro, entries }: Props = $props();
</script>

<div class="tw-ds tw-licenses">
  <h2 class="value-md" style:color="var(--text-primary)">{title}</h2>
  {#if intro}
    <p class="body" style:color="var(--text-secondary)">{intro}</p>
  {/if}
  <div class="list">
    {#each entries as entry (entry.id)}
      <details class="entry">
        <summary>
          <span class="chevron" aria-hidden="true">
            <svg viewBox="0 0 12 12" width="10" height="10"><path d="M4 2 L8 6 L4 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </span>
          <span class="body-strong name" style:color="var(--text-primary)">{entry.name}</span>
          {#if entry.version}
            <span class="caption" style:color="var(--text-tertiary)">{entry.version}</span>
          {/if}
          <span class="caption license">{entry.license}</span>
        </summary>
        <pre class="text">{entry.text}</pre>
      </details>
    {/each}
  </div>
</div>

<style>
  .tw-licenses {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 720px;
    margin: 0 auto;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  h2,
  p {
    margin: 0;
  }
  .list {
    display: flex;
    flex-direction: column;
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .entry {
    border-bottom: 1px solid var(--hairline);
  }
  .entry:last-child {
    border-bottom: none;
  }
  summary {
    list-style: none;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    cursor: default;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }
  .chevron {
    display: flex;
    color: var(--text-tertiary);
    transition: transform 160ms ease;
  }
  .entry[open] .chevron {
    transform: rotate(90deg);
  }
  .name {
    flex: 1;
    min-width: 0;
  }
  .license {
    color: var(--accent-blue);
    padding: 2px 8px;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-full);
  }
  .text {
    margin: 0;
    padding: var(--space-3) var(--space-4) var(--space-4) calc(var(--space-4) + 22px);
    color: var(--text-secondary);
    font-family: ui-monospace, 'Cascadia Mono', Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--surface-sunken);
  }
</style>
