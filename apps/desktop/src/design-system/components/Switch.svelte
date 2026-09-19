<script lang="ts">
  /**
   * A boolean toggle — Ajustes is full of these (background monitoring,
   * notifications, persistence, anonymization, the general
   * notifications switch, opt-in auto-update checks, "iniciar oculto
   * en la bandeja"). This is the control only; pair it with
   * OptionRow for the label + description + disabled-reason layout
   * the settings screen actually needs.
   *
   * `checked` is bindable:
   *
   *   <Switch bind:checked={notificationsEnabled} label="Notificaciones" />
   *
   * When `disabled`, still render the control (never hide it) and let
   * the consumer explain the condition via OptionRow's `disabledReason`
   * — a disabled control with no explanation is exactly what the
   * spec's "controles con dependencias no desaparecen" rule forbids.
   */
  interface Props {
    checked?: boolean;
    label: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let { checked = $bindable(false), label, disabled = false, onchange }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  class="tw-switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={toggle}
>
  <span class="thumb"></span>
</button>

<style>
  .tw-switch {
    width: 36px;
    height: 22px;
    padding: 2px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--surface-sunken);
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    flex: 0 0 auto;
    cursor: default;
  }
  .tw-switch.on {
    background: var(--accent-blue);
    justify-content: flex-end;
  }
  .tw-switch:disabled {
    opacity: 0.4;
    pointer-events: none;
  }
  .tw-switch:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }
  .thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #ffffff;
  }
</style>
