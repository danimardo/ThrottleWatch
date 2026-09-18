<script lang="ts">
  /**
   * Reference composition exercising every state `SessionsScreen`
   * accepts — NOT a component this system exports. A demo control
   * panel switches the screen-level status (loading/error/loaded) and
   * whether the loaded list is empty; the seeded sessions below cover
   * every per-card status the spec lists.
   */
  import { SessionsScreen, NavIcon } from '../components';
  import type { SessionsListStatus, SessionsScreenSession } from '../components';
  import { SegmentedControl, Switch } from '../components';

  let screenStatus: SessionsListStatus = $state('loaded');
  let forceEmpty = $state(false);
  let selectedId = $state('s2');
  let lastAction = $state('(ninguna todavía)');
  let deletedIds: string[] = $state([]);

  const seedSessions: SessionsScreenSession[] = [
    {
      id: 's1',
      status: 'active',
      typeLabel: 'Monitorización continua',
      dateLabel: 'Hoy, 14:32',
      durationLabel: 'En curso',
      statusLabel: 'En curso',
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión activa (s1)')
    },
    {
      id: 's2',
      status: 'completed',
      typeLabel: 'Diagnóstico guiado',
      dateLabel: '16 sept 2026, 09:10',
      durationLabel: '14 min',
      classification: 'thermal_confirmed',
      classificationLabel: 'Limitación térmica confirmada',
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión completada (s2)'),
      referenceLabel: 'Referencia',
      referenceActionLabel: 'Usar como referencia',
      onToggleReference: () => (lastAction = 'usar s2 como referencia'),
      exportLabel: 'Exportar',
      onExport: () => (lastAction = 'exportar s2'),
      deleteLabel: 'Eliminar'
    },
    {
      id: 's3',
      status: 'completed',
      typeLabel: 'Diagnóstico guiado',
      dateLabel: '15 sept 2026, 18:45',
      durationLabel: '11 min',
      classification: 'normal',
      classificationLabel: 'Sin limitación',
      isReference: true,
      referenceLabel: 'Referencia',
      referenceActionLabel: 'Retirar referencia',
      onToggleReference: () => (lastAction = 'retirar referencia de s3'),
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión completada (s3)'),
      exportLabel: 'Exportar',
      onExport: () => (lastAction = 'exportar s3'),
      deleteLabel: 'Eliminar'
    },
    {
      id: 's4',
      status: 'cancelled',
      typeLabel: 'Diagnóstico guiado',
      dateLabel: '14 sept 2026, 20:02',
      durationLabel: '2 min',
      statusLabel: 'Cancelada',
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión cancelada (s4)'),
      exportLabel: 'Exportar',
      onExport: () => (lastAction = 'exportar s4'),
      exportDisabled: true,
      exportDisabledReason: 'No hay datos suficientes en una sesión cancelada.',
      deleteLabel: 'Eliminar'
    },
    {
      id: 's5',
      status: 'incomplete',
      typeLabel: 'Monitorización continua',
      dateLabel: '13 sept 2026, 11:20',
      durationLabel: '38 min',
      statusLabel: 'Incompleta — el equipo se suspendió',
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión incompleta (s5)'),
      exportLabel: 'Exportar',
      onExport: () => (lastAction = 'exportar s5'),
      exportDisabled: true,
      exportDisabledReason: 'La sesión se interrumpió antes de completar el análisis.',
      deleteLabel: 'Eliminar'
    },
    {
      id: 's6',
      status: 'imported',
      typeLabel: 'Sesión importada',
      dateLabel: '2 sept 2026, 08:00',
      durationLabel: '9 min',
      classification: 'power_limited',
      classificationLabel: 'Limitación eléctrica',
      importedLabel: 'Importada',
      openLabel: 'Abrir',
      onOpen: () => (lastAction = 'abrir sesión importada (s6)'),
      exportLabel: 'Exportar',
      onExport: () => (lastAction = 'exportar s6'),
      deleteLabel: 'Eliminar'
    }
  ];

  let sessions: SessionsScreenSession[] = $derived.by(() =>
    seedSessions
      .filter((s) => !deletedIds.includes(s.id))
      .map((s) => ({ ...s, selected: s.id === selectedId, onOpen: () => (selectedId = s.id) }))
  );

  function handleDelete(id: string) {
    deletedIds = [...deletedIds, id];
    lastAction = 'eliminar sesión ' + id;
  }
</script>

{#snippet emptyIcon()}
  <NavIcon kind="sessions" />
{/snippet}

<div class="tw-ds sessions-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">ESTADO DE PANTALLA</span>
      <SegmentedControl
        label="Estado de pantalla"
        value={screenStatus}
        onchange={(v) => (screenStatus = v as SessionsListStatus)}
        options={[
          { value: 'loaded', label: 'Cargada' },
          { value: 'loading', label: 'Cargando' },
          { value: 'error', label: 'Error' }
        ]}
      />
    </div>
    <div class="control">
      <Switch bind:checked={forceEmpty} label="Forzar lista vacía" />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <SessionsScreen
      status={screenStatus}
      sessions={forceEmpty ? [] : sessions}
      title="Sesiones"
      importLabel="Importar…"
      onImport={() => (lastAction = 'importar sesión (selector nativo)')}
      onDeleteSession={handleDelete}
      loadingLabel="Cargando sesiones…"
      emptyTitle="Todavía no hay sesiones"
      emptyDescription="Inicia un diagnóstico guiado o activa la monitorización continua para ver tu historial aquí."
      {emptyIcon}
      errorTitle="No se pudo cargar el historial de sesiones"
      errorDescription="Comprueba el almacenamiento local e inténtalo de nuevo."
      retryLabel="Reintentar"
      onRetry={() => {
        lastAction = 'reintentar carga de sesiones';
        screenStatus = 'loaded';
      }}
      deleteDialogTitle="Eliminar sesión"
      deleteDialogDescription="Se borrará esta sesión y su informe asociado. Esta acción no se puede deshacer."
      deleteDialogCancelLabel="Cancelar"
      deleteDialogConfirmLabel="Eliminar"
    />
  </div>
</div>

<style>
  .sessions-demo {
    background: var(--bg);
    padding: var(--space-6);
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif);
  }
  .control-panel {
    width: 220px;
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
  .screen-frame {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  /* This stacking is this DEMO's own layout responding to the browser
     window (deciding where to put its control panel) — not
     SessionsScreen itself, which reflows purely via its own
     @container query (see SessionCard.svelte). */
  @media (max-width: 900px) {
    .sessions-demo {
      flex-direction: column;
    }
    .control-panel {
      width: auto;
      position: static;
    }
    .screen-frame {
      width: 100%;
    }
  }
</style>
