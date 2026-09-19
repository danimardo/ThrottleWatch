<script lang="ts">
  /**
   * Custom window chrome for the main window (and onboarding / startup
   * error dialogs). Tauri publishes with no OS window decoration — this
   * bar IS the decoration. Deliberately NOT the macOS traffic-light
   * control style: this is a Windows app, and the three controls are
   * monochrome icons of equal visual weight.
   *
   * This component owns no window-lifecycle logic — Rust/Tauri does.
   * It only reports intent through the on* callbacks; wire them to
   * @tauri-apps/api/window, e.g.:
   *
   *   import { getCurrentWindow } from '@tauri-apps/api/window';
   *   const win = getCurrentWindow();
   *   <TitleBar
   *     title="ThrottleWatch"
   *     maximized={await win.isMaximized()}
   *     onMinimize={() => win.minimize()}
   *     onMaximizeToggle={() => win.toggleMaximize()}
   *     onClose={() => win.close()}
   *     minimizeLabel="Minimizar"
   *     maximizeLabel="Maximizar"
   *     restoreLabel="Restaurar"
   *     closeLabel="Cerrar"
   *   />
   *
   * The region between the mark and the controls is a full Tauri drag
   * region (data-tauri-drag-region) — double-clicking it also toggles
   * maximize/restore, matching native behavior.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    maximized?: boolean;
    onMinimize: () => void;
    onMaximizeToggle: () => void;
    onClose: () => void;
    /** Optional app mark rendered before the title. Omit for text-only. */
    icon?: Snippet;
    minimizeLabel: string;
    maximizeLabel: string;
    restoreLabel: string;
    closeLabel: string;
  }

  let {
    title,
    maximized = false,
    onMinimize,
    onMaximizeToggle,
    onClose,
    icon,
    minimizeLabel,
    maximizeLabel,
    restoreLabel,
    closeLabel
  }: Props = $props();
</script>

<div class="tw-titlebar">
  {#if icon}
    <span class="mark">{@render icon()}</span>
  {/if}
  <span class="name label">{title}</span>
  <div
    class="drag"
    data-tauri-drag-region
    ondblclick={onMaximizeToggle}
    role="presentation"
  ></div>
  <div class="winctl">
    <button type="button" aria-label={minimizeLabel} onclick={onMinimize}>
      <svg class="wc" viewBox="0 0 10 10"><rect x="0" y="4.5" width="10" height="1.1" rx=".5" fill="currentColor" /></svg>
    </button>
    <button type="button" aria-label={maximized ? restoreLabel : maximizeLabel} onclick={onMaximizeToggle}>
      {#if maximized}
        <svg class="wc" viewBox="0 0 10 10">
          <rect x="1.6" y="0.6" width="6.8" height="6.8" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.1" />
          <path d="M0.6 2.6 V8.4 A1 1 0 0 0 1.6 9.4 H7.4" fill="none" stroke="currentColor" stroke-width="1.1" />
        </svg>
      {:else}
        <svg class="wc" viewBox="0 0 10 10"><rect x="0.6" y="0.6" width="8.8" height="8.8" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.1" /></svg>
      {/if}
    </button>
    <button type="button" class="close" aria-label={closeLabel} onclick={onClose}>
      <svg class="wc" viewBox="0 0 10 10"><path d="M0.7 0.7 L9.3 9.3 M9.3 0.7 L0.7 9.3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
    </button>
  </div>
</div>

<style>
  .tw-titlebar {
    display: flex;
    align-items: center;
    height: 38px;
    padding: 0 6px 0 12px;
    background-color: var(--glass-bg-strong);
    background-image: var(--glass-sheen);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-edge);
    position: relative;
    z-index: 4;
    border-bottom: 1px solid var(--hairline);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .mark {
    width: 15px;
    height: 15px;
    margin-right: 7px;
    flex: 0 0 auto;
    display: flex;
  }
  .mark :global(svg) {
    width: 100%;
    height: 100%;
  }
  .name {
    color: var(--text-primary);
    letter-spacing: -0.005em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .drag {
    flex: 1;
    min-width: 8px;
    height: 100%;
  }
  .winctl {
    display: flex;
    gap: 0;
  }
  .winctl button {
    width: 36px;
    height: 38px;
    min-width: 36px;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0;
    padding: 0;
    cursor: default;
  }
  .winctl button:hover {
    background: var(--surface-raised);
  }
  .winctl button:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }
  /*
    The close button is the only surface in the whole app that tints
    red without representing a thermal state — kept deliberately subtle
    so it never competes with a real thermal alert elsewhere in the UI.
  */
  .winctl button.close:hover {
    background: color-mix(in srgb, var(--status-thermal) 14%, transparent);
    color: var(--status-thermal);
  }
  svg.wc {
    width: 9px;
    height: 9px;
  }
</style>
