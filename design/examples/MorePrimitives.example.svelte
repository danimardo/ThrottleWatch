<script lang="ts">
  /**
   * Reference composition for the 4 "batch 4" primitives — NOT a
   * component this system exports. Demonstrates: Tooltip on a CPU
   * topology-map-style core tile (the exact use case that motivated
   * it — see Tooltip.svelte's own doc comment), Select inside an
   * Ajustes-style OptionRow, ProgressBar both determinate (an update
   * download) and indeterminate (a diagnóstico guiado step still
   * gathering its first reading), and Banner + EmptyState.
   */
  import { OptionRow, Select, Tooltip, ProgressBar, Banner, EmptyState, NavIcon, Button } from '../components';

  const cores = [
    { id: 'P0', temp: 87, load: 74, clock: 4.4, throttle: 0 },
    { id: 'P1', temp: 91, load: 88, clock: 4.2, throttle: 0 },
    { id: 'P2', temp: 98, load: 92, clock: 4.1, throttle: 12 },
    { id: 'E0', temp: 79, load: 61, clock: 3.3, throttle: 0 }
  ];

  let language = $state('system');

  let downloadPercent = $state(42);

  let bannerDismissed = $state(false);
</script>

<div class="tw-ds more-demo">
  <section>
    <h3 class="label" style:color="var(--text-tertiary)">TOOLTIP</h3>
    <div class="core-row">
      {#each cores as core (core.id)}
        <Tooltip label={`${core.temp}°C · Carga ${core.load}% · ${core.clock}GHz${core.throttle ? ` · throttling ${core.throttle}s` : ''}`}>
          {#snippet children({ describedBy })}
            <button class="core-tile body-strong" aria-describedby={describedBy}>{core.id}</button>
          {/snippet}
        </Tooltip>
      {/each}
    </div>
  </section>

  <section>
    <h3 class="label" style:color="var(--text-tertiary)">SELECT</h3>
    <OptionRow label="Idioma" description="Cambia el idioma de toda la aplicación.">
      {#snippet control()}
        <Select
          label="Idioma"
          bind:value={language}
          options={[
            { value: 'system', label: 'Usar idioma del sistema' },
            { value: 'es', label: 'Español' },
            { value: 'en', label: 'English' },
            { value: 'fr', label: 'Français' },
            { value: 'de', label: 'Deutsch' }
          ]}
        />
      {/snippet}
    </OptionRow>
  </section>

  <section>
    <h3 class="label" style:color="var(--text-tertiary)">PROGRESSBAR</h3>
    <div class="progress-stack">
      <div>
        <span class="body" style:color="var(--text-secondary)">Descargando actualización — {downloadPercent}%</span>
        <ProgressBar percent={downloadPercent} tone="accent" label="Descargando actualización" />
      </div>
      <div>
        <span class="body" style:color="var(--text-secondary)">Diagnóstico guiado — reuniendo la primera lectura</span>
        <ProgressBar indeterminate tone="power" label="Reuniendo la primera lectura" />
      </div>
    </div>
  </section>

  <section>
    <h3 class="label" style:color="var(--text-tertiary)">BANNER</h3>
    <div class="banner-stack">
      <Banner tone="warning" title="Modo de bajo consumo activo" description="Algunas lecturas de sensores se omiten mientras el equipo está en bajo consumo." />
      {#if !bannerDismissed}
        <Banner
          tone="critical"
          title="No se pudo conectar con el sensor de energía"
          description="ThrottleWatch seguirá mostrando lecturas térmicas. Reintenta o revisa el controlador del sensor."
          onDismiss={() => (bannerDismissed = true)}
          dismissLabel="Cerrar"
        >
          {#snippet action()}
            <Button variant="secondary" label="Reintentar" />
          {/snippet}
        </Banner>
      {/if}
    </div>
  </section>

  <section>
    <h3 class="label" style:color="var(--text-tertiary)">EMPTYSTATE</h3>
    <div class="empty-frame">
      <EmptyState title="Aún no hay historial suficiente" description="Vuelve dentro de unos minutos: Análisis necesita varias lecturas antes de mostrar tendencias.">
        {#snippet icon()}
          <NavIcon kind="analysis" />
        {/snippet}
        {#snippet action()}
          <Button variant="secondary" label="Actualizar ahora" />
        {/snippet}
      </EmptyState>
    </div>
  </section>
</div>

<style>
  .more-demo {
    background: var(--bg);
    padding: var(--space-6);
    width: 520px;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  section {
    margin-bottom: var(--space-6);
  }
  h3 {
    margin: 0 0 var(--space-3) 0;
  }
  .core-row {
    display: flex;
    gap: var(--space-3);
  }
  .core-tile {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    border: 1px solid var(--hairline);
    color: var(--text-primary);
  }
  .core-tile:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
  .progress-stack,
  .banner-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .progress-stack .body {
    display: block;
    margin-bottom: 6px;
  }
  .empty-frame {
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
  }
</style>
