<script lang="ts">
  /**
   * Replaces the sidebar when the window drops below 700px wide (see
   * lib/responsive.svelte.ts / README breakpoint table). The compact
   * bar shows exactly four slots: three direct destinations plus a
   * "Más" item that opens a small menu with the remaining ones, so
   * that nothing (Sesiones, Diagnóstico guiado, Ajustes) becomes
   * unreachable in compact width — ux-visual-spec → "Navegación".
   *
   * NON-NEGOTIABLE (functional spec, not a design preference): "Ahora"
   * is always present and always the first icon. If you ever need to
   * add another compact destination, put it in `menu.items` — in dev
   * this component warns if you pass more than 3 direct items (4
   * including the menu), as a reminder to re-check that rule, but it
   * does not reorder items for you.
   *
   * Active state is icon + label color only (accent-blue) — no
   * background, no extra indicator, to keep the bar light at this
   * small a size. The "Más" item is highlighted while any of its menu
   * items is active, so the person can tell the current screen is
   * "behind" the menu.
   *
   * The menu's open/closed state is view-only local state (same
   * category as Select's popup). Escape closes it and returns focus.
   */
  import type { Snippet } from 'svelte';

  export interface BottomBarItem {
    id: string;
    label: string;
    icon: Snippet;
    active?: boolean;
    onclick?: () => void;
  }

  export interface BottomBarMenuItem {
    id: string;
    label: string;
    icon?: Snippet;
    active?: boolean;
    onclick?: () => void;
  }

  export interface BottomBarMenu {
    id: string;
    /** Visible caption under the icon, e.g. "Más". */
    label: string;
    icon: Snippet;
    /** Accessible name for the popup list, e.g. "Más destinos". */
    menuLabel: string;
    items: BottomBarMenuItem[];
  }

  interface Props {
    items: BottomBarItem[];
    menu?: BottomBarMenu;
  }

  let { items, menu }: Props = $props();

  let menuOpen = $state(false);
  let menuButton: HTMLButtonElement | undefined = $state();
  let menuAnyActive = $derived(menu?.items.some((i) => i.active) ?? false);

  $effect(() => {
    const max = menu ? 3 : 4;
    if (import.meta.env?.DEV && items.length > max) {
      console.warn(
        `[ThrottleWatch/BottomBar] ${items.length} direct items passed — the compact bar shows at most ${max}${menu ? ' plus the menu' : ''}. ` +
          'Move extra destinations into `menu.items` instead of crowding this bar.'
      );
    }
  });

  function toggleMenu() {
    menuOpen = !menuOpen;
  }
  function closeMenu(returnFocus = true) {
    if (!menuOpen) return;
    menuOpen = false;
    if (returnFocus) menuButton?.focus();
  }
  function pick(item: BottomBarMenuItem) {
    closeMenu(false);
    item.onclick?.();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      closeMenu();
    }
  }
</script>

<svelte:window onkeydown={menuOpen ? onKeydown : undefined} />

<div class="bottombar-wrap">
  {#if menu && menuOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="scrim" onclick={() => closeMenu()} aria-hidden="true"></div>
    <div class="menu" role="menu" aria-label={menu.menuLabel}>
      {#each menu.items as item (item.id)}
        <button
          type="button"
          role="menuitem"
          class="menu-item body"
          class:active={item.active}
          aria-current={item.active ? 'page' : undefined}
          onclick={() => pick(item)}
        >
          {#if item.icon}
            <span class="menu-icon">{@render item.icon()}</span>
          {/if}
          <span>{item.label}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="bottombar">
    {#each items as item (item.id)}
      <button
        type="button"
        class="bitem"
        class:active={item.active}
        aria-current={item.active ? 'page' : undefined}
        onclick={() => {
          closeMenu(false);
          item.onclick?.();
        }}
      >
        <span class="icon">{@render item.icon()}</span>
        <span class="caption">{item.label}</span>
      </button>
    {/each}
    {#if menu}
      <button
        bind:this={menuButton}
        type="button"
        class="bitem"
        class:active={menuAnyActive}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={toggleMenu}
      >
        <span class="icon">{@render menu.icon()}</span>
        <span class="caption">{menu.label}</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .bottombar-wrap {
    position: relative;
  }
  .bottombar {
    display: flex;
    align-items: center;
    justify-content: space-around;
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border-top: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    padding: 8px 4px 10px;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    position: relative;
    z-index: 2;
  }
  .bitem {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    flex: 1;
    padding: 2px 0;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
  }
  .bitem .icon {
    width: 20px;
    height: 20px;
    display: flex;
    transition: transform var(--motion-base) var(--motion-spring);
  }
  .bitem:active .icon {
    transform: scale(0.88);
  }
  .bitem.active .icon {
    animation: tw-bump var(--motion-slow) var(--motion-spring);
  }
  .bitem .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .bitem.active {
    color: var(--accent-blue);
  }
  .bitem:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
    border-radius: 6px;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
  }
  .menu {
    position: absolute;
    right: var(--space-3);
    bottom: calc(100% + var(--space-2));
    min-width: 200px;
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow-strong);
    border-radius: var(--radius-md);
    z-index: 3;
    transform-origin: 90% 100%;
    animation: tw-drop var(--motion-base) var(--motion-spring) both;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-primary);
    border-radius: var(--radius-sm);
    text-align: left;
  }
  .menu-item:hover {
    background: var(--surface-sunken);
  }
  .menu-item.active {
    color: var(--accent-blue);
  }
  .menu-item:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }
  .menu-icon {
    width: 18px;
    height: 18px;
    display: flex;
  }
  .menu-icon :global(svg) {
    width: 100%;
    height: 100%;
  }
</style>
