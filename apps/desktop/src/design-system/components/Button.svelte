<script lang="ts">
  /**
   * The generic action button this system was missing: onboarding
   * navigation (Atrás/Siguiente/Omitir/Abrir Ahora), "Iniciar
   * diagnóstico" / "Detener ahora", dialog confirm/cancel, "Buscar
   * actualizaciones". Deliberately NOT the marketing-pill CTA shape —
   * a rounded rectangle (`radius-sm`) at native button proportions,
   * matching macOS/iOS system buttons rather than a web "Get started"
   * button. No gradient, no shadow (this system has none).
   *
   * Three variants:
   * - "primary"     solid accent-blue — the one main action on a screen
   *                 or dialog (Abrir Ahora, Iniciar diagnóstico, the
   *                 default action in a confirmation dialog).
   * - "secondary"    tinted surface-raised + hairline border — every
   *                 other action (Atrás, Omitir, Cancelar).
   * - "destructive"  solid status-thermal — reserved for the ONE
   *                 confirm button inside a Dialog that finalizes an
   *                 irreversible action (Restablecer ThrottleWatch,
   *                 Eliminar todos mis datos). Do not use it for the
   *                 row/entry-point button that opens that dialog —
   *                 that row uses "secondary" and lives inside a
   *                 clearly labeled "risk zone" section instead. This
   *                 keeps status-thermal meaning "thermal limitation"
   *                 everywhere outside an active confirmation, per the
   *                 system's own "the same color means the same class
   *                 everywhere" rule.
   *
   * The solid fills are a deepened color-mix of the token, not the raw
   * token: white label text on the raw accent-blue/status-thermal
   * dark-theme values only clears ~3.5:1, short of 4.5:1 text
   * contrast. Darkening by ~12-16% toward black restores AA in both
   * themes with one formula (see the style block below) — do not swap
   * these back to var(--accent-blue) / var(--status-thermal)
   * directly for the background.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    variant?: 'primary' | 'secondary' | 'destructive';
    icon?: Snippet;
    disabled?: boolean;
    type?: 'button' | 'submit';
    onclick?: () => void;
  }

  let { label, variant = 'primary', icon, disabled = false, type = 'button', onclick }: Props = $props();

  // Liquid ripple: the press point becomes the origin of an expanding
  // highlight (CSS-only, driven by two custom properties). Decorative;
  // collapses to nothing under reduced motion via tokens.css.
  let rx = $state(50);
  let ry = $state(50);
  let rippleKey = $state(0);
  function onPointerDown(e: PointerEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    rx = ((e.clientX - r.left) / r.width) * 100;
    ry = ((e.clientY - r.top) / r.height) * 100;
    rippleKey++;
  }
</script>

<button {type} class="tw-btn {variant}" {disabled} {onclick} onpointerdown={onPointerDown} style:--tw-rx="{rx}%" style:--tw-ry="{ry}%">
  {#key rippleKey}
    {#if rippleKey > 0}<span class="ripple" aria-hidden="true"></span>{/if}
  {/key}
  {#if icon}
    <span class="icon">{@render icon()}</span>
  {/if}
  <span class="body-strong">{label}</span>
</button>

<style>
  .tw-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
    cursor: default;
    position: relative;
    overflow: hidden;
    isolation: isolate;
    transition:
      transform var(--motion-fast) var(--motion-spring),
      background-color var(--motion-fast) linear,
      box-shadow var(--motion-base) var(--motion-ease-out);
  }
  .tw-btn:active:not(:disabled) {
    transform: scale(0.97);
  }
  .ripple {
    position: absolute;
    left: var(--tw-rx, 50%);
    top: var(--tw-ry, 50%);
    width: 260%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(255, 255, 255, 0.55) 0%, rgba(255, 255, 255, 0.18) 35%, transparent 70%);
    transform: translate(-50%, -50%) scale(0);
    transform-origin: center;
    pointer-events: none;
    animation: tw-btn-ripple var(--motion-slow) var(--motion-ease-out) both;
    z-index: -1;
  }
  .tw-btn.secondary .ripple {
    background: radial-gradient(circle, color-mix(in srgb, var(--accent-blue) 30%, transparent) 0%, transparent 70%);
  }
  @keyframes tw-btn-ripple {
    from {
      transform: translate(-50%, -50%) scale(0);
      opacity: 0.9;
    }
    to {
      transform: translate(-50%, -50%) scale(1);
      opacity: 0;
    }
  }
  .tw-btn:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }
  .tw-btn:disabled {
    opacity: 0.4;
    pointer-events: none;
  }
  .icon {
    width: 15px;
    height: 15px;
    display: flex;
    flex: 0 0 auto;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }

  /* Deepened fills: see doc comment above for why this isn't var(--accent-blue) directly. */
  .tw-btn.primary {
    background: color-mix(in srgb, var(--accent-blue) 88%, black);
    color: #ffffff;
  }
  .tw-btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent-blue) 78%, black);
  }
  .tw-btn.destructive {
    background: color-mix(in srgb, var(--status-thermal) 84%, black);
    color: #ffffff;
  }
  .tw-btn.destructive:hover:not(:disabled) {
    background: color-mix(in srgb, var(--status-thermal) 74%, black);
  }
  .tw-btn.secondary {
    background-color: var(--glass-bg-subtle);
    background-image: var(--glass-sheen);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-edge);
    -webkit-backdrop-filter: var(--glass-filter);
    backdrop-filter: var(--glass-filter);
    color: var(--text-primary);
  }
  .tw-btn.secondary:hover:not(:disabled) {
    background-color: var(--glass-bg-strong);
    box-shadow: var(--glass-shadow);
  }
  .tw-btn.primary,
  .tw-btn.destructive {
    box-shadow: var(--glass-edge);
  }
</style>
