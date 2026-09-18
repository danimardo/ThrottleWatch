<script lang="ts">
  /**
   * A dropdown for choosing one of a larger set of options — reach for
   * this only when you have too many options to fit comfortably as
   * segments (see SegmentedControl's own doc comment for where that
   * line sits: 2–5 mutually-exclusive choices stay a SegmentedControl;
   * a longer list — many languages, a longer retention-period list —
   * is this instead).
   *
   * Built as a custom listbox, not a native <select>, because a native
   * select's popup is painted by the OS/browser outside any CSS this
   * system controls — on Windows that is a literal system dropdown with
   * its own font and colors, which breaks the "consistently themed
   * native app" illusion the whole system is built around. This one is
   * fully themeable in both dark and light, and keyboard-operable:
   * ArrowUp/ArrowDown move the highlighted option, Enter/Space choose
   * it, Escape closes and returns focus to the trigger.
   *
   * `value` is bindable:
   *
   *   <Select bind:value={language} label="Idioma"
   *     options={[{value:'system',label:'Usar idioma del sistema'}, ...]} />
   */
  interface SelectOption {
    value: string;
    label: string;
  }

  interface Props {
    options: SelectOption[];
    value?: string;
    label: string;
    disabled?: boolean;
    onchange?: (value: string) => void;
  }

  let { options, value = $bindable(options[0]?.value ?? ''), label, disabled = false, onchange }: Props = $props();

  const id = `tw-select-${Math.random().toString(36).slice(2, 9)}`;
  let open = $state(false);
  let activeIndex = $state(0);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let listEl: HTMLUListElement | undefined = $state();

  let selectedIndex = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
  let selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? '');

  function openList() {
    if (disabled) return;
    activeIndex = selectedIndex;
    open = true;
  }
  function closeList(returnFocus = true) {
    open = false;
    if (returnFocus) triggerEl?.focus();
  }
  function choose(i: number) {
    const opt = options[i];
    if (!opt) return;
    value = opt.value;
    onchange?.(opt.value);
    closeList();
  }
  function onTriggerKeydown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (!open) openList();
      else if (e.key === 'Enter' || e.key === ' ') choose(activeIndex);
    }
  }
  function onListKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = Math.min(options.length - 1, activeIndex + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = Math.max(0, activeIndex - 1);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      choose(activeIndex);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeList();
    } else if (e.key === 'Tab') {
      closeList(false);
    }
  }
  function onWindowClick(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    if (triggerEl?.contains(target) || listEl?.contains(target)) return;
    closeList(false);
  }

  $effect(() => {
    if (open) {
      window.addEventListener('click', onWindowClick);
      listEl?.focus();
      return () => window.removeEventListener('click', onWindowClick);
    }
  });
</script>

<div class="tw-select-wrap">
  <button
    type="button"
    bind:this={triggerEl}
    class="tw-select-trigger body"
    class:open
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={id}
    aria-label={label}
    onclick={() => (open ? closeList() : openList())}
    onkeydown={onTriggerKeydown}
  >
    <span class="value">{selectedLabel}</span>
    <span class="chevron" aria-hidden="true">
      <svg viewBox="0 0 12 12" width="10" height="10">
        <path d="M2.5 4.5 L6 8 L9.5 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </span>
  </button>

  {#if open}
    <ul
      bind:this={listEl}
      {id}
      role="listbox"
      class="tw-select-list"
      tabindex="-1"
      aria-activedescendant={`${id}-opt-${activeIndex}`}
      aria-label={label}
      onkeydown={onListKeydown}
    >
      {#each options as opt, i (opt.value)}
        <li
          id={`${id}-opt-${i}`}
          role="option"
          aria-selected={opt.value === value}
          class="tw-select-option body"
          class:active={i === activeIndex}
          class:selected={opt.value === value}
          onpointerenter={() => (activeIndex = i)}
          onclick={() => choose(i)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') choose(i);
          }}
        >
          <span class="opt-label">{opt.label}</span>
          {#if opt.value === value}
            <svg class="check" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
              <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .tw-select-wrap {
    position: relative;
    display: inline-block;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .tw-select-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-width: 160px;
    padding: 7px 10px 7px 12px;
    background: var(--surface-raised);
    color: var(--text-primary);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-sm);
  }
  .tw-select-trigger:disabled {
    opacity: 0.4;
    pointer-events: none;
  }
  .tw-select-trigger:focus-visible,
  .tw-select-trigger.open {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .tw-select-trigger .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tw-select-trigger .chevron {
    flex: 0 0 auto;
    display: flex;
    color: var(--text-tertiary);
  }

  .tw-select-list {
    animation: tw-drop var(--motion-base) var(--motion-spring) both;
    transform-origin: top center;
    position: absolute;
    z-index: 30;
    top: calc(100% + 6px);
    left: 0;
    min-width: 100%;
    max-height: 260px;
    overflow-y: auto;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-md);
  }
  .tw-select-list:focus-visible {
    outline: none;
  }
  .tw-select-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    white-space: nowrap;
  }
  .tw-select-option.active {
    background: var(--surface-sunken);
  }
  .tw-select-option .opt-label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tw-select-option .check {
    flex: 0 0 auto;
    color: var(--accent-blue);
  }
</style>
