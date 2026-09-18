<script lang="ts">
  /**
   * Reference composition for the 5 base primitives added after the
   * first 8 components — NOT a component this system exports. Shows
   * a slice of "Ajustes" (Apariencia + Bandeja/notificaciones + a
   * risk-zone reset with its confirmation dialog) built entirely from
   * OptionRow + Switch + SegmentedControl + Button + Dialog.
   */
  import { OptionRow, Switch, SegmentedControl, Button, Dialog } from '../components';

  let theme = $state('system');
  let trayMonitoring = $state(true);
  let notifications = $state(false);
  let startHidden = $state(false);
  let resetOpen = $state(false);
</script>

<div class="tw-ds settings-demo">
  <section>
    <h3 class="label" style:color="var(--text-tertiary)">APARIENCIA</h3>
    <OptionRow label="Tema" description="«Sistema» sigue el tema de Windows en caliente.">
      {#snippet control()}
        <SegmentedControl
          label="Tema"
          bind:value={theme}
          options={[
            { value: 'system', label: 'Sistema' },
            { value: 'light', label: 'Claro' },
            { value: 'dark', label: 'Oscuro' }
          ]}
        />
      {/snippet}
    </OptionRow>
  </section>

  <section>
    <h3 class="label" style:color="var(--text-tertiary)">BANDEJA Y NOTIFICACIONES</h3>
    <OptionRow label="Monitorización en segundo plano" description="Sigue leyendo sensores con la ventana cerrada.">
      {#snippet control()}
        <Switch bind:checked={trayMonitoring} label="Monitorización en segundo plano" />
      {/snippet}
    </OptionRow>
    <OptionRow label="Notificaciones" description="Gobierna los avisos térmicos y eléctricos.">
      {#snippet control()}
        <Switch bind:checked={notifications} label="Notificaciones" />
      {/snippet}
    </OptionRow>
    <OptionRow
      label="Iniciar oculto en la bandeja"
      disabled={!trayMonitoring}
      disabledReason="Requiere activar la monitorización en bandeja."
    >
      {#snippet control()}
        <Switch bind:checked={startHidden} disabled={!trayMonitoring} label="Iniciar oculto en la bandeja" />
      {/snippet}
    </OptionRow>
  </section>

  <section class="risk-zone">
    <h3 class="label" style:color="var(--status-thermal)">ZONA DE RIESGO</h3>
    <OptionRow label="Restablecer ThrottleWatch" description="Borra preferencias y datos. La próxima vez se repetirá el primer inicio.">
      {#snippet control()}
        <Button variant="secondary" label="Restablecer…" onclick={() => (resetOpen = true)} />
      {/snippet}
    </OptionRow>
  </section>

  <Dialog
    bind:open={resetOpen}
    title="Restablecer ThrottleWatch"
    description="Se borrarán las preferencias y los datos guardados. La próxima vez que abras la app se repetirá el primer inicio. Esta acción no se puede deshacer."
    tone="warning"
  >
    {#snippet actions()}
      <Button variant="secondary" label="Cancelar" onclick={() => (resetOpen = false)} />
      <Button variant="destructive" label="Restablecer" onclick={() => (resetOpen = false)} />
    {/snippet}
  </Dialog>
</div>

<style>
  .settings-demo {
    background: var(--bg);
    padding: var(--space-6);
    width: 480px;
  }
  section {
    margin-bottom: var(--space-6);
  }
  h3 {
    margin: 0 0 var(--space-2) 0;
  }
  .risk-zone {
    border: 1px solid color-mix(in srgb, var(--status-thermal) 30%, var(--hairline));
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
  }
</style>
