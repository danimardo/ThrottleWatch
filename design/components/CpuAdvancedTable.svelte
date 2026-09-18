<script lang="ts">
  /**
   * The per-core advanced table: every core as a row, filterable (by
   * core number or group) and sortable (click a header), and
   * virtualized so a many-core machine (64, 128 threads) never renders
   * more DOM rows than fit the visible window. This system ships zero
   * runtime dependencies, so the virtualization is a small hand-rolled
   * windowed render (track `scrollTop`, render only the rows the
   * viewport can show plus a few rows of overscan, pad the rest with
   * two spacer elements) rather than pulling in a virtualization
   * library.
   *
   * Sorting/filtering are generic list operations this component owns
   * itself (like `Select`'s own open/closed state) — they are not the
   * kind of domain judgment (a classification, a diagnosis) this
   * system otherwise insists stays out of its components. The host
   * still supplies both the display label AND the raw sortable value
   * for temperature/clock/load (`temperatureValue`, etc.) — this
   * component sorts numbers, it does not parse "78°" back into 78.
   *
   * A core with `unavailable` still gets a row (never dropped by the
   * filter unless its number/group doesn't match) — its numeric
   * columns render the row's label text as-is (expected to already
   * read as a placeholder, e.g. "—", from the host).
   */
  export interface CoreTableRow {
    id: string;
    index: number;
    groupLabel?: string;
    temperatureLabel: string;
    temperatureValue?: number;
    clockLabel: string;
    clockValue?: number;
    loadLabel: string;
    loadValue?: number;
    throttlingLabel: string;
    unavailable?: boolean;
  }

  export type CoreTableSortColumn = 'index' | 'temperature' | 'clock' | 'load';
  export type CoreTableSortDirection = 'asc' | 'desc';

  export interface CoreTableColumnLabels {
    index: string;
    group: string;
    temperature: string;
    clock: string;
    load: string;
    throttling: string;
  }

  interface Props {
    rows: CoreTableRow[];
    columnLabels: CoreTableColumnLabels;
    filterLabel: string;
    filterPlaceholder?: string;
    emptyFilterMessage: string;
    rowHeight?: number;
    maxHeight?: number;
  }

  let { rows, columnLabels, filterLabel, filterPlaceholder, emptyFilterMessage, rowHeight = 36, maxHeight = 320 }: Props = $props();

  let filterText = $state('');
  let sortColumn: CoreTableSortColumn = $state('index');
  let sortDirection: CoreTableSortDirection = $state('asc');
  let scrollTop = $state(0);
  let viewportEl: HTMLDivElement | undefined = $state();

  let filteredRows = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((r) => String(r.index).includes(q) || (r.groupLabel ?? '').toLowerCase().includes(q));
  });

  let sortedRows = $derived.by(() => {
    const dir = sortDirection === 'asc' ? 1 : -1;
    function key(r: CoreTableRow): number {
      if (sortColumn === 'temperature') return r.temperatureValue ?? -Infinity;
      if (sortColumn === 'clock') return r.clockValue ?? -Infinity;
      if (sortColumn === 'load') return r.loadValue ?? -Infinity;
      return r.index;
    }
    return [...filteredRows].sort((a, b) => (key(a) - key(b)) * dir);
  });

  const OVERSCAN = 4;
  let visibleCount = $derived(Math.ceil(maxHeight / rowHeight) + OVERSCAN * 2);
  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN));
  let endIndex = $derived(Math.min(sortedRows.length, startIndex + visibleCount));
  let visibleRows = $derived(sortedRows.slice(startIndex, endIndex));
  let topSpacer = $derived(startIndex * rowHeight);
  let bottomSpacer = $derived(Math.max(0, (sortedRows.length - endIndex) * rowHeight));

  function toggleSort(col: CoreTableSortColumn) {
    if (sortColumn === col) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = col;
      sortDirection = 'asc';
    }
  }
  function onScroll() {
    if (viewportEl) scrollTop = viewportEl.scrollTop;
  }
  function sortIndicator(col: CoreTableSortColumn): string {
    if (sortColumn !== col) return '';
    return sortDirection === 'asc' ? '▲' : '▼';
  }
</script>

<div class="tw-cpu-table">
  <div class="filter-row">
    <label class="filter-label caption" style:color="var(--text-tertiary)" for="tw-cpu-table-filter">{filterLabel}</label>
    <input
      id="tw-cpu-table-filter"
      class="filter-input body"
      type="text"
      placeholder={filterPlaceholder}
      bind:value={filterText}
    />
  </div>

  <div class="scroll-area">
    <div class="header-row">
      <button type="button" class="col col-index caption" onclick={() => toggleSort('index')}>{columnLabels.index} {sortIndicator('index')}</button>
      <span class="col col-group caption">{columnLabels.group}</span>
      <button type="button" class="col col-num caption" onclick={() => toggleSort('temperature')}>{columnLabels.temperature} {sortIndicator('temperature')}</button>
      <button type="button" class="col col-num caption" onclick={() => toggleSort('clock')}>{columnLabels.clock} {sortIndicator('clock')}</button>
      <button type="button" class="col col-num caption" onclick={() => toggleSort('load')}>{columnLabels.load} {sortIndicator('load')}</button>
      <span class="col col-throttle caption">{columnLabels.throttling}</span>
    </div>

    {#if sortedRows.length === 0}
      <div class="empty-filter body" style:color="var(--text-tertiary)">{emptyFilterMessage}</div>
    {:else}
      <div class="viewport" bind:this={viewportEl} style:max-height={`${maxHeight}px`} onscroll={onScroll}>
        <div style:height={`${topSpacer}px`}></div>
        {#each visibleRows as row (row.id)}
          <div class="data-row" class:unavailable={row.unavailable} style:height={`${rowHeight}px`}>
            <span class="col col-index body">{row.index}</span>
            <span class="col col-group body">{row.groupLabel ?? ''}</span>
            <span class="col col-num body">{row.temperatureLabel}</span>
            <span class="col col-num body">{row.clockLabel}</span>
            <span class="col col-num body">{row.loadLabel}</span>
            <span class="col col-throttle body">{row.throttlingLabel}</span>
          </div>
        {/each}
        <div style:height={`${bottomSpacer}px`}></div>
      </div>
    {/if}
  </div>
</div>

<style>
  .tw-cpu-table {
    background-color: var(--glass-bg);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .filter-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .filter-input {
    flex: 1;
    max-width: 220px;
    padding: 6px 10px;
    background: var(--surface-raised);
    color: var(--text-primary);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-sm);
  }
  .filter-input:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .scroll-area {
    overflow-x: auto;
  }
  .header-row,
  .data-row {
    display: grid;
    grid-template-columns: 56px 64px 1fr 1fr 1fr 96px;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    min-width: 540px;
  }
  .header-row {
    border-bottom: 1px solid var(--hairline);
    padding-bottom: var(--space-2);
  }
  .col {
    text-align: left;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button.col {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-tertiary);
  }
  button.col:hover {
    color: var(--text-primary);
  }
  button.col:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .data-row {
    border-bottom: 1px solid var(--hairline);
  }
  .data-row.unavailable .col {
    color: var(--text-tertiary);
  }
  .viewport {
    overflow-y: auto;
  }
  .empty-filter {
    padding: var(--space-5) var(--space-2);
    text-align: center;
  }
</style>
