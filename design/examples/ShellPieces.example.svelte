<script lang="ts">
  /**
   * Reference composition for the pieces added after the 2026-09-18
   * spec review (gap-analysis.md §10): `ContextStrip`, `CoverageMatrix`,
   * `ExportDialog`, `ImportResultDialog`, `FirstCloseDialog`,
   * `CloseBlockedDialog`, `WhatsNewCards`, `TechnicalSummary`,
   * `LicensesScreen`, and the `BottomBar` "Más" menu. NOT a component
   * this system exports — a demo panel opens each dialog and flips the
   * strip's collector state. Hardcoded Spanish for demo brevity only.
   */
  import {
    ContextStrip,
    CoverageMatrix,
    ExportDialog,
    ImportResultDialog,
    FirstCloseDialog,
    CloseBlockedDialog,
    WhatsNewCards,
    TechnicalSummary,
    LicensesScreen,
    BottomBar,
    Banner,
    Button,
    NavIcon,
    SegmentedControl,
    Select
  } from '../components';
  import type {
    CollectorState,
    CoverageRow,
    ExportFormat,
    ImportStatus,
    CloseBlockedReason,
    WhatsNewCard,
    AdvancedAccessState
  } from '../components';

  let collectorState: CollectorState = $state('fresh');
  let accessState: AdvancedAccessState = $state('installable');
  let lastAction = $state('(ninguna todavía)');

  // dialogs
  let exportOpen = $state(false);
  let exportFormat: ExportFormat = $state('csv');
  let exportAnonymize = $state(true);
  let importOpen = $state(false);
  let importStatus: ImportStatus = $state('success');
  let firstCloseOpen = $state(false);
  let blockedOpen = $state(false);
  let blockedReason: CloseBlockedReason = $state('guided');

  // what's new
  let cards: WhatsNewCard[] = $state([
    { id: 'n1', title: 'El acceso avanzado ahora se instala por separado', body: 'ThrottleWatch ya no pide permisos de administrador al abrirse. Instálalo desde Ajustes → Sensores para confirmar la causa y estimar cuánto ayudaría enfriar mejor.', actionLabel: 'Ir a Ajustes', onAction: () => (lastAction = 'ir a Ajustes desde novedades') },
    { id: 'n2', title: 'Nuevo aviso: prueba guiada terminada', body: 'Puedes recibir una notificación cuando un diagnóstico acabe con la ventana en la bandeja. Está apagado de fábrica.' }
  ]);

  let copied = $state(false);
  let active = $state('now');

  const COLLECTOR_LABELS: Record<CollectorState, string> = {
    fresh: 'Conectado · hace 1 s',
    stale: 'Datos obsoletos · hace 12 s',
    disconnected: 'Colector desconectado',
    starting: 'Iniciando colector…'
  };
  const ACCESS_NOTES: Record<AdvancedAccessState, string> = {
    not_needed: 'No hace falta acceso avanzado en este equipo.',
    available: 'Acceso avanzado disponible.',
    installable: 'Recomendado: el acceso avanzado sube este equipo al nivel A (confirmar la causa y estimar cuánto ayudaría enfriar mejor).',
    upgradable: 'Hay una versión anterior del controlador de acceso avanzado; actualizarla mejora la cobertura.',
    denied: 'El acceso avanzado está bloqueado por una directiva del sistema o por el antivirus.',
    error: 'No se pudo comprobar el acceso avanzado.'
  };

  let coverageRows: CoverageRow[] = $derived.by(() => [
    { id: 'temperature', label: 'Temperatura', available: true, quality: 'direct', qualityLabel: 'Directo', sourceLabel: 'CPU Package' },
    { id: 'headroom', label: 'Margen térmico', available: true, quality: 'derived', qualityLabel: 'Derivado', sourceLabel: 'TjMax 100 °C − paquete' },
    { id: 'load', label: 'Carga', available: true, quality: 'direct', qualityLabel: 'Directo', sourceLabel: 'CPU Total' },
    { id: 'clock', label: 'Frecuencia activa', available: true, quality: 'derived', qualityLabel: 'Derivado', sourceLabel: '% Processor Performance × base' },
    { id: 'base', label: 'Frecuencia base', available: true, quality: 'derived', qualityLabel: 'Derivado', sourceLabel: 'Processor Frequency' },
    { id: 'power', label: 'Potencia', available: true, quality: 'substitute', qualityLabel: 'Sustituto', sourceLabel: 'Estimación por carga', reasonLabel: '' },
    { id: 'thermal_flag', label: 'Razón térmica', available: accessState === 'available', quality: 'direct', qualityLabel: 'Directo', sourceLabel: accessState === 'available' ? 'IA32_THERM_STATUS' : undefined, reasonLabel: 'Requiere acceso avanzado.' },
    { id: 'power_flag', label: 'Razón de potencia', available: accessState === 'available', quality: 'direct', qualityLabel: 'Directo', sourceLabel: accessState === 'available' ? 'MSR_CORE_PERF_LIMIT_REASONS' : undefined, reasonLabel: 'Requiere acceso avanzado.' }
  ]);

  let includedFields = $derived.by(() =>
    exportFormat === 'csv'
      ? ['timestamp_utc', 'monotonic_ms', 'sensor_id', 'metric', 'scope', 'value', 'status', 'quality', 'cpu.vendor', 'cpu.display_name', 'cpu.topology', 'versions']
      : ['classification', 'confidence_band', 'evidence', 'alternative_causes', 'events', 'cooling_potential', 'coverage_tier', 'ruleset_version', 'cpu.vendor', 'cpu.display_name', 'cpu.topology']
  );
  let excludedFields = $derived.by(() =>
    exportAnonymize
      ? ['hostname', 'usuario de Windows', 'números de serie', 'direcciones MAC', 'rutas locales', 'huella de monitores', 'GUID del plan de energía']
      : []
  );

  function simulateCopy() {
    copied = true;
    lastAction = 'copiar resumen técnico';
    setTimeout(() => (copied = false), 1200);
  }
</script>

{#snippet nowIcon()}<NavIcon kind="now" />{/snippet}
{#snippet analysisIcon()}<NavIcon kind="analysis" />{/snippet}
{#snippet cpuIcon()}<NavIcon kind="cpu" />{/snippet}
{#snippet sessionsIcon()}<NavIcon kind="sessions" />{/snippet}
{#snippet guidedIcon()}<NavIcon kind="guided" />{/snippet}
{#snippet settingsIcon()}<NavIcon kind="settings" />{/snippet}
{#snippet moreIcon()}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="5" cy="12" r="1.4" fill="currentColor" /><circle cx="12" cy="12" r="1.4" fill="currentColor" /><circle cx="19" cy="12" r="1.4" fill="currentColor" /></svg>
{/snippet}
{#snippet stopAction()}
  <Button variant="secondary" label="Detener" onclick={() => (lastAction = 'detener prueba desde la barra global')} />
{/snippet}
{#snippet collectorActions()}
  <div style="display:flex; gap: var(--space-2); flex-wrap: wrap;">
    <Button variant="secondary" label="Reintentar" onclick={() => (lastAction = 'reintentar colector')} />
    <Button variant="secondary" label="Ver resumen técnico" onclick={() => (lastAction = 'ver resumen técnico')} />
  </div>
{/snippet}

<div class="tw-ds shell-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">COLECTOR</span>
      <Select
        label="Estado del colector"
        value={collectorState}
        onchange={(v) => (collectorState = v as CollectorState)}
        options={[
          { value: 'fresh', label: 'Fresco' },
          { value: 'stale', label: 'Obsoleto' },
          { value: 'disconnected', label: 'Desconectado' },
          { value: 'starting', label: 'Iniciando' }
        ]}
      />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">ACCESO AVANZADO</span>
      <Select
        label="Acceso avanzado"
        value={accessState}
        onchange={(v) => (accessState = v as AdvancedAccessState)}
        options={[
          { value: 'not_needed', label: 'No hace falta' },
          { value: 'available', label: 'Disponible' },
          { value: 'installable', label: 'Instalable' },
          { value: 'denied', label: 'Bloqueado' },
          { value: 'error', label: 'Error' }
        ]}
      />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">DIÁLOGOS</span>
      <Button variant="secondary" label="ExportDialog" onclick={() => (exportOpen = true)} />
      <Button variant="secondary" label="ImportResultDialog" onclick={() => (importOpen = true)} />
      <SegmentedControl
        label="Estado de importación"
        value={importStatus}
        onchange={(v) => (importStatus = v as ImportStatus)}
        options={[
          { value: 'validating', label: 'Validando' },
          { value: 'success', label: 'OK' },
          { value: 'error', label: 'Error' }
        ]}
      />
      <Button variant="secondary" label="FirstCloseDialog" onclick={() => (firstCloseOpen = true)} />
      <Button variant="secondary" label="CloseBlockedDialog" onclick={() => (blockedOpen = true)} />
      <Select
        label="Motivo de bloqueo"
        value={blockedReason}
        onchange={(v) => (blockedReason = v as CloseBlockedReason)}
        options={[
          { value: 'guided', label: 'Prueba en curso' },
          { value: 'export', label: 'Exportación' },
          { value: 'download', label: 'Descarga' },
          { value: 'install', label: 'Instalación' }
        ]}
      />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="stage">
    <section class="piece">
      <span class="caption piece-title">Barras globales bajo la barra de título (prueba en curso · colector degradado)</span>
      <Banner tone="warning" title="Prueba en curso · Carga sostenida · quedan 1 min 21 s" action={stopAction} />
      <Banner tone="critical" title="El colector de sensores se ha detenido" description="Reintentando (2 de 3). Los datos mostrados están obsoletos." action={collectorActions} />
    </section>

    <section class="piece">
      <span class="caption piece-title">ContextStrip</span>
      <ContextStrip
        cpuLabel="Intel Core Ultra 7 155H"
        topologyLabel="6P + 8E + 2LP"
        powerLabel="Batería 64 % · Equilibrado"
        {collectorState}
        collectorLabel={COLLECTOR_LABELS[collectorState]}
        coverageActionLabel="Ver cobertura"
        onCoverage={() => (lastAction = 'abrir panel de cobertura')}
      />
      <ContextStrip
        compact
        cpuLabel="AMD Ryzen 7 7840HS"
        powerLabel="CA · Rendimiento"
        {collectorState}
        collectorLabel={COLLECTOR_LABELS[collectorState]}
        coverageActionLabel="Ver cobertura"
        onCoverage={() => (lastAction = 'abrir panel de cobertura (compacto)')}
      />
    </section>

    <section class="piece">
      <span class="caption piece-title">CoverageMatrix</span>
      <CoverageMatrix
        title="Cobertura de este equipo"
        rows={coverageRows}
        columnLabels={{ magnitude: 'Magnitud', available: 'Disponible', quality: 'Calidad', source: 'Origen', reason: 'Motivo' }}
        availableLabel="Sí"
        unavailableLabel="No"
        maxConfidenceLabel={accessState === 'available'
          ? 'Nivel A · completo — confirma la causa y estima el potencial. Confianza máxima alcanzable: alta'
          : 'Nivel B · con potencia — infiere la causa sin confirmarla. Confianza máxima alcanzable: media'}
        {accessState}
        accessLabel={ACCESS_NOTES[accessState]}
        requestAccessLabel="Instalar acceso avanzado"
        onRequestAccess={() => {
          accessState = 'available';
          lastAction = 'instalar acceso avanzado (UAC)';
        }}
        accessRetryLabel="Reintentar"
        onAccessRetry={() => (accessState = 'available')}
        recheckLabel="Volver a comprobar"
        onRecheck={() => (lastAction = 'volver a comprobar')}
        copySummaryLabel="Copiar resumen técnico"
        onCopySummary={simulateCopy}
      />
    </section>

    <section class="piece">
      <span class="caption piece-title">WhatsNewCards</span>
      <WhatsNewCards
        title="Novedades de la versión 1.5"
        {cards}
        dismissLabel="Entendido"
        onDismiss={(id) => {
          cards = cards.filter((c) => c.id !== id);
          lastAction = 'novedad descartada: ' + id;
        }}
        dismissAllLabel="Descartar todas"
        onDismissAll={() => (cards = [])}
      />
      {#if cards.length === 0}
        <Button variant="secondary" label="Restaurar novedades" onclick={() => (cards = [
          { id: 'n1', title: 'El acceso avanzado ahora se instala por separado', body: 'ThrottleWatch ya no pide permisos de administrador al abrirse.', actionLabel: 'Ir a Ajustes', onAction: () => (lastAction = 'ir a Ajustes') },
          { id: 'n2', title: 'Nuevo aviso: prueba guiada terminada', body: 'Está apagado de fábrica.' }
        ])} />
      {/if}
    </section>

    <section class="piece">
      <span class="caption piece-title">TechnicalSummary</span>
      <TechnicalSummary
        title="Resumen técnico"
        note="No contiene identificadores del equipo ni del usuario."
        regionLabel="Resumen técnico"
        text={`ThrottleWatch 1.4.2 (build 20340) · protocolo IPC v1 · sensor-agent 1.4.2 (.NET 10)
colector: ${collectorState} · reinicios (10 min): 0 · lag p95: 31 ms
acceso avanzado: ${accessState} · ruleset: v1 · último error: —`}
        copyLabel="Copiar"
        copiedLabel="Copiado"
        {copied}
        onCopy={simulateCopy}
      />
    </section>

    <section class="piece">
      <span class="caption piece-title">LicensesScreen</span>
      <LicensesScreen
        title="Licencias de terceros"
        intro="ThrottleWatch incluye software de terceros con sus propias licencias."
        entries={[
          { id: 'lhm', name: 'LibreHardwareMonitorLib', version: '0.9.5', license: 'MPL-2.0', text: 'Mozilla Public License Version 2.0\n\n1. Definitions\n1.1. "Contributor" means each individual or legal entity that creates, contributes to the creation of, or owns Covered Software. …' },
          { id: 'tauri', name: 'Tauri', version: '2.x', license: 'MIT / Apache-2.0', text: 'Permission is hereby granted, free of charge, to any person obtaining a copy of this software …' },
          { id: 'svelte', name: 'Svelte', version: '5.x', license: 'MIT', text: 'Copyright (c) 2016-2026 Svelte contributors …' }
        ]}
      />
    </section>

    <section class="piece">
      <span class="caption piece-title">BottomBar con «Más»</span>
      <div class="phone">
        <BottomBar
          items={[
            { id: 'now', label: 'Ahora', icon: nowIcon, active: active === 'now', onclick: () => (active = 'now') },
            { id: 'analysis', label: 'Análisis', icon: analysisIcon, active: active === 'analysis', onclick: () => (active = 'analysis') },
            { id: 'cpu', label: 'CPU', icon: cpuIcon, active: active === 'cpu', onclick: () => (active = 'cpu') }
          ]}
          menu={{
            id: 'more',
            label: 'Más',
            icon: moreIcon,
            menuLabel: 'Más destinos',
            items: [
              { id: 'sessions', label: 'Sesiones', icon: sessionsIcon, active: active === 'sessions', onclick: () => (active = 'sessions') },
              { id: 'guided', label: 'Diagnóstico guiado', icon: guidedIcon, active: active === 'guided', onclick: () => (active = 'guided') },
              { id: 'settings', label: 'Ajustes', icon: settingsIcon, active: active === 'settings', onclick: () => (active = 'settings') }
            ]
          }}
        />
      </div>
      <span class="caption" style:color="var(--text-tertiary)">Destino activo: {active}</span>
    </section>
  </div>
</div>

<ExportDialog
  bind:open={exportOpen}
  title="Exportar sesión"
  scopeLabel="Sesión del 16 sept 2026, 09:10 · Diagnóstico guiado · 14 min"
  formatLabel="Formato"
  format={exportFormat}
  formatOptions={[
    { value: 'csv', label: 'CSV de muestras' },
    { value: 'json', label: 'JSON de informe' }
  ]}
  onFormatChange={(f) => (exportFormat = f)}
  anonymizeLabel="Anonimizar"
  anonymizeDescription="Elimina identificadores del equipo y del usuario."
  anonymize={exportAnonymize}
  onAnonymizeChange={(v) => (exportAnonymize = v)}
  includedTitle="Se incluye"
  {includedFields}
  excludedTitle="Se excluye"
  {excludedFields}
  sizeLabel={exportFormat === 'csv' ? 'Tamaño estimado: 1,8 MB' : 'Tamaño estimado: 42 KB'}
  fileNameLabel={exportFormat === 'csv' ? 'throttlewatch_session_2026-09-16_a1b2c3.csv' : 'throttlewatch_report_2026-09-16_a1b2c3.json'}
  cancelLabel="Cancelar"
  confirmLabel="Elegir carpeta y exportar"
  onCancel={() => (exportOpen = false)}
  onConfirm={() => {
    exportOpen = false;
    lastAction = `exportar ${exportFormat} (${exportAnonymize ? 'anónimo' : 'completo'}) → diálogo nativo`;
  }}
/>

<ImportResultDialog
  bind:open={importOpen}
  title="Importar sesión"
  status={importStatus}
  progressLabel="Validando el fichero…"
  description={importStatus === 'error'
    ? 'El fichero usa un formato más reciente que esta versión de ThrottleWatch (schema 3). Actualiza la aplicación para importarlo.'
    : 'Sesión importada: 12 sept 2026, 11 min · Diagnóstico guiado (schema 1, migrado a 2).'}
  warningsTitle="Avisos"
  warnings={importStatus === 'success' ? ['La referencia incluida en el fichero no se usará para este equipo.', 'El informe original se conserva; puedes reevaluarlo con las reglas actuales.'] : []}
  closeLabel="Cerrar"
  onClose={() => (importOpen = false)}
  openSessionLabel="Abrir sesión"
  onOpenSession={() => {
    importOpen = false;
    lastAction = 'abrir sesión importada';
  }}
/>

<FirstCloseDialog
  bind:open={firstCloseOpen}
  title="¿Qué quieres hacer al cerrar?"
  description="Puedes salir de ThrottleWatch o dejarla en la bandeja del sistema para que siga midiendo."
  trayNote="Elegir la bandeja activa la monitorización en segundo plano."
  settingsHint="Puedes cambiarlo cuando quieras en Ajustes → General."
  exitLabel="Salir de ThrottleWatch"
  trayLabel="Continuar en la bandeja y seguir midiendo"
  onExit={() => {
    firstCloseOpen = false;
    lastAction = 'primera X: salir';
  }}
  onTray={() => {
    firstCloseOpen = false;
    lastAction = 'primera X: bandeja (activa monitorización)';
  }}
  onDismiss={() => {
    firstCloseOpen = false;
    lastAction = 'primera X: cerrado sin decidir';
  }}
/>

<CloseBlockedDialog
  bind:open={blockedOpen}
  reason={blockedReason}
  title={blockedReason === 'guided'
    ? 'Hay un diagnóstico en curso'
    : blockedReason === 'export'
      ? 'Hay una exportación en curso'
      : blockedReason === 'download'
        ? 'Hay una descarga en curso'
        : 'Instalando una actualización'}
  description={blockedReason === 'guided'
    ? 'Si sales ahora, la prueba se detendrá y los datos parciales se guardarán como incompletos.'
    : blockedReason === 'export'
      ? 'Se esperará hasta 5 segundos a que termine; después se cancelará.'
      : blockedReason === 'download'
        ? 'La descarga se cancelará y el fichero parcial se descartará.'
        : 'ThrottleWatch se cerrará sola cuando el instalador esté listo. No se puede cerrar durante la instalación.'}
  confirmLabel={blockedReason === 'guided' ? 'Detener y salir' : blockedReason === 'install' ? undefined : 'Salir de todos modos'}
  onConfirm={blockedReason === 'install'
    ? undefined
    : () => {
        blockedOpen = false;
        lastAction = `cierre bloqueado (${blockedReason}): confirmar`;
      }}
  cancelLabel={blockedReason === 'install' ? 'Entendido' : 'Cancelar'}
  onCancel={() => {
    blockedOpen = false;
    lastAction = `cierre bloqueado (${blockedReason}): cancelar`;
  }}
/>

<style>
  .shell-demo {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .control-panel {
    width: 240px;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    position: sticky;
    top: var(--space-6);
  }
  .control {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .last-action {
    padding-top: var(--space-2);
    border-top: 1px solid var(--hairline);
  }
  .stage {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }
  .piece {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .piece-title {
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .phone {
    width: 360px;
    max-width: 100%;
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    padding-top: 120px;
    background: var(--surface-sunken);
    overflow: visible;
  }
  @media (max-width: 900px) {
    .shell-demo {
      flex-direction: column;
    }
    .control-panel {
      width: auto;
      position: static;
    }
  }
</style>
