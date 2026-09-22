<script lang="ts">
  /**
   * Reference composition exercising every domain state
   * `SettingsScreen` accepts — NOT a component this system exports
   * (the component itself IS the export; this file only demonstrates
   * driving it). A demo control panel above the screen lets you flip:
   * the tri-state close action, dependency chains (background
   * monitoring → "iniciar oculto", notifications → "avisar al
   * terminar"), sensor coverage (detecting/complete/partial + the
   * 5-state advanced access), the update flow's full status machine
   * including the separate download → verified → install gestures,
   * and the two destructive confirmations (borrado de datos,
   * restablecimiento) including their in-progress and error states.
   *
   * Option values mirror data-model.md's preference keys on purpose
   * (`session|1d|7d|30d`, `keep|low_power|pause`, `short|standard|long`,
   * `system|reduced|full`) so the example doubles as a reminder of
   * the real vocabulary.
   */
  import { SettingsScreen, CoverageMatrix, TechnicalSummary } from '../components';
  import type {
    SettingsUpdateStatus,
    SettingsSensorStatus,
    SettingsDangerStatus,
    SettingsCloseAction,
    AdvancedAccessState,
    CoverageRow
  } from '../components';
  import { SegmentedControl, Select, Switch } from '../components';

  // --- General ---
  let closeAction: SettingsCloseAction = $state('unset');
  let startOnLogin = $state(false);
  let startHiddenInTray = $state(false);

  // --- Idioma ---
  let language = $state('system');

  // --- Apariencia ---
  let theme = $state('system');
  let motion = $state('system');
  let glassPref = $state('system');

  // --- Monitorización ---
  let monitoringMode = $state('normal');
  let onBattery = $state('keep');
  let samplingInterval = $state('1000');
  let perCoreHistory = $state(false);

  // --- Bandeja y notificaciones ---
  let backgroundMonitoring = $state(false);
  let notifications = $state(false);
  let quietEnabled = $state(false);
  let quietStart = $state('22');
  let quietEnd = $state('8');

  // --- Datos y privacidad ---
  let anonymizeExports = $state(true);
  let dataRetention = $state('7d');
  let deleteStatus: SettingsDangerStatus = $state('idle');

  // --- Sensores y cobertura (demo controls) ---
  let sensorStatus: SettingsSensorStatus = $state('partial');
  let advancedAccess: AdvancedAccessState = $state('installable');
  let showMatrix = $state(true);

  // --- Diagnóstico ---
  let duration = $state('standard');
  let requireAc = $state(false);
  let notifyOnFinish = $state(false);

  // --- Actualizaciones (demo controls) ---
  let autoCheck = $state(false);
  let updateStatus: SettingsUpdateStatus = $state('idle');
  let downloadPercent = $state(0);
  let installBlocked = $state(false);

  // --- Acerca de ---
  let detailedLogging = $state(false);
  let copied = $state(false);

  // --- Zona de riesgo ---
  let resetStatus: SettingsDangerStatus = $state('idle');

  let lastAction = $state('(ninguna todavía)');

  const ACCESS_NOTES: Record<AdvancedAccessState, string> = {
    not_needed: 'No hace falta acceso avanzado en este equipo.',
    available: 'Acceso avanzado disponible: razones de limitación, límites de potencia y TCC offset.',
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
    { id: 'power', label: 'Potencia', available: sensorStatus === 'complete', quality: 'direct', qualityLabel: 'Directo', sourceLabel: sensorStatus === 'complete' ? 'CPU Package Power' : undefined, reasonLabel: 'El sensor de potencia no está expuesto por la placa base.' },
    { id: 'thermal_flag', label: 'Razón térmica', available: advancedAccess === 'available', quality: 'direct', qualityLabel: 'Directo', sourceLabel: advancedAccess === 'available' ? 'IA32_THERM_STATUS' : undefined, reasonLabel: 'Requiere acceso avanzado.' },
    { id: 'power_flag', label: 'Razón de potencia', available: advancedAccess === 'available', quality: 'direct', qualityLabel: 'Directo', sourceLabel: advancedAccess === 'available' ? 'MSR_CORE_PERF_LIMIT_REASONS' : undefined, reasonLabel: 'Requiere acceso avanzado.' }
  ]);

  function simulateCheckUpdate() {
    updateStatus = 'checking';
    lastAction = 'buscar actualizaciones';
    setTimeout(() => (updateStatus = 'available'), 700);
  }
  function simulateDownload() {
    updateStatus = 'downloading';
    downloadPercent = 0;
    lastAction = 'descargar actualización';
    const timer = setInterval(() => {
      downloadPercent = Math.min(100, downloadPercent + 20);
      if (downloadPercent >= 100) {
        clearInterval(timer);
        updateStatus = 'verified';
      }
    }, 220);
  }
  function simulateInstall() {
    updateStatus = 'installing';
    lastAction = 'instalar (cierra la aplicación)';
    setTimeout(() => (updateStatus = 'upToDate'), 900);
  }
  function simulateDelete() {
    deleteStatus = 'inProgress';
    lastAction = 'eliminar todos mis datos';
    setTimeout(() => (deleteStatus = 'idle'), 900);
  }
  function simulateReset() {
    resetStatus = 'inProgress';
    lastAction = 'restablecer ThrottleWatch';
    setTimeout(() => (resetStatus = 'idle'), 900);
  }
  function simulateCopy() {
    copied = true;
    lastAction = 'copiar resumen técnico';
    setTimeout(() => (copied = false), 1200);
  }
</script>

{#snippet coverageSnippet()}
  <CoverageMatrix
    rows={coverageRows}
    columnLabels={{ magnitude: 'Magnitud', available: 'Disponible', quality: 'Calidad', source: 'Origen', reason: 'Motivo' }}
    availableLabel="Sí"
    unavailableLabel="No"
    maxConfidenceLabel={advancedAccess === 'available'
      ? 'Nivel A · completo — confirma la causa y estima el potencial. Confianza máxima alcanzable: alta'
      : 'Nivel B · con potencia — infiere la causa sin confirmarla. Confianza máxima alcanzable: media'}
    accessState={advancedAccess}
    accessLabel={ACCESS_NOTES[advancedAccess]}
    requestAccessLabel="Instalar acceso avanzado"
    onRequestAccess={() => {
      advancedAccess = 'available';
      lastAction = 'instalar acceso avanzado (UAC)';
    }}
    accessRetryLabel="Reintentar"
    onAccessRetry={() => {
      advancedAccess = 'available';
      lastAction = 'reintentar acceso avanzado';
    }}
    recheckLabel="Volver a comprobar"
    recheckDisabled={sensorStatus === 'detecting'}
    onRecheck={() => {
      sensorStatus = 'detecting';
      lastAction = 'volver a comprobar sensores';
      setTimeout(() => (sensorStatus = 'complete'), 900);
    }}
    copySummaryLabel="Copiar resumen técnico"
    onCopySummary={simulateCopy}
  />
{/snippet}

{#snippet technicalSummarySnippet()}
  <TechnicalSummary
    title="Resumen técnico"
    note="No contiene identificadores del equipo ni del usuario."
    regionLabel="Resumen técnico"
    text={`ThrottleWatch 1.4.2 (build 20340) · protocolo IPC v1 · sensor-agent 1.4.2 (.NET 10)
colector: running · reinicios (10 min): 0 · lag p95: 31 ms
cobertura: temperatura=direct headroom=derived load=direct clock=derived power=${sensorStatus === 'complete' ? 'direct' : 'missing'} thermal_flag=${advancedAccess === 'available' ? 'direct' : 'missing'}
acceso avanzado: ${advancedAccess} · ruleset: v1 · último error: —
almacenamiento: 38 MB · retención: ${dataRetention} · perfil: ${monitoringMode}`}
    copyLabel="Copiar"
    copiedLabel="Copiado"
    {copied}
    onCopy={simulateCopy}
  />
{/snippet}

<div class="tw-ds settings-screen-demo">
  <div class="control-panel">
    <span class="label" style:color="var(--text-tertiary)">DEMO — ESTADOS</span>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">SENSORES</span>
      <SegmentedControl
        label="Estado de sensores"
        value={sensorStatus}
        onchange={(v) => (sensorStatus = v as SettingsSensorStatus)}
        options={[
          { value: 'detecting', label: 'Detectando' },
          { value: 'complete', label: 'Completa' },
          { value: 'partial', label: 'Parcial' }
        ]}
      />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">ACCESO AVANZADO</span>
      <Select
        label="Acceso avanzado"
        value={advancedAccess}
        onchange={(v) => (advancedAccess = v as AdvancedAccessState)}
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
      <Switch bind:checked={showMatrix} label="Matriz de cobertura embebida" />
    </div>
    <div class="control">
      <span class="caption" style:color="var(--text-tertiary)">ACTUALIZACIONES</span>
      <Select
        label="Estado de actualización"
        value={updateStatus}
        onchange={(v) => {
          updateStatus = v as SettingsUpdateStatus;
          if (v === 'error') lastAction = 'forzar error de actualización';
        }}
        options={[
          { value: 'idle', label: 'Inactivo' },
          { value: 'upToDate', label: 'Al día' },
          { value: 'available', label: 'Disponible (descargar)' },
          { value: 'verified', label: 'Verificada (instalar)' },
          { value: 'error', label: 'Error' }
        ]}
      />
    </div>
    <div class="control">
      <Switch bind:checked={installBlocked} label="Instalación bloqueada (diagnóstico en curso)" />
    </div>
    <span class="body last-action" style:color="var(--text-secondary)">Última acción: {lastAction}</span>
  </div>

  <div class="screen-frame">
    <SettingsScreen
      general={{
        title: 'General',
        closeActionRowLabel: 'Al cerrar la ventana',
        closeActionRowDescription: 'Salir cierra ThrottleWatch; Bandeja sigue midiendo en segundo plano.',
        closeActionLabel: 'Acción al cerrar',
        closeAction,
        closeActionOptions: [
          { value: 'exit', label: 'Salir' },
          { value: 'tray', label: 'Bandeja' }
        ],
        closeActionUndecidedText: 'Aún no has decidido: se te preguntará la primera vez que cierres la ventana.',
        onCloseActionChange: (v) => {
          closeAction = v;
          if (v === 'tray') backgroundMonitoring = true;
          lastAction = 'acción al cerrar: ' + v;
        },
        startOnLogin,
        startOnLoginLabel: 'Iniciar con Windows',
        startOnLoginDescription: 'Abre ThrottleWatch automáticamente al iniciar sesión.',
        onStartOnLoginChange: (v) => {
          startOnLogin = v;
          lastAction = 'iniciar con Windows: ' + v;
        },
        startHiddenLabel: 'Iniciar oculto en la bandeja',
        startHiddenDescription: 'La ventana no aparece al iniciar; solo el icono de bandeja.',
        startHiddenInTray,
        onStartHiddenInTrayChange: (v) => (startHiddenInTray = v),
        startHiddenDisabled: !startOnLogin || !backgroundMonitoring,
        startHiddenDisabledReason: !startOnLogin
          ? 'Requiere «Iniciar con Windows».'
          : 'Requiere activar la monitorización en segundo plano.'
      }}
      language={{
        title: 'Idioma',
        rowLabel: 'Idioma de la aplicación',
        rowDescription: '«Usar idioma del sistema» sigue el primer idioma de visualización de Windows; el cambio se aplica al siguiente inicio.',
        selectLabel: 'Idioma',
        value: language,
        options: [
          { value: 'system', label: 'Usar idioma del sistema' },
          { value: 'es', label: 'Español' },
          { value: 'en', label: 'English' }
        ],
        onChange: (v) => (language = v),
        effectiveLabel: language === 'system' ? 'Idioma efectivo: Español (Windows: es-ES)' : undefined
      }}
      appearance={{
        title: 'Apariencia',
        rowLabel: 'Tema',
        rowDescription: '«Sistema» sigue el modo de aplicación de Windows en caliente.',
        segmentedLabel: 'Tema',
        theme,
        options: [
          { value: 'system', label: 'Sistema' },
          { value: 'light', label: 'Claro' },
          { value: 'dark', label: 'Oscuro' }
        ],
        onThemeChange: (v) => (theme = v),
        motionRowLabel: 'Movimiento',
        motionRowDescription: '«Reducido» elimina interpolaciones y halos; «Sistema» sigue la preferencia de Windows.',
        motionLabel: 'Movimiento',
        motion,
        motionOptions: [
          { value: 'system', label: 'Sistema' },
          { value: 'reduced', label: 'Reducido' },
          { value: 'full', label: 'Completo' }
        ],
        onMotionChange: (v) => (motion = v),
        glassRowLabel: 'Efecto de vidrio',
        glassRowDescription: '«Sistema» sigue los efectos de transparencia de Windows; «Reducido» quita el desenfoque; «Sin» usa superficies sólidas.',
        glassLabel: 'Efecto de vidrio',
        glass: glassPref,
        glassOptions: [
          { value: 'system', label: 'Sistema' },
          { value: 'full', label: 'Completo' },
          { value: 'reduced', label: 'Reducido' },
          { value: 'off', label: 'Sin' }
        ],
        onGlassChange: (v) => (glassPref = v)
      }}
      monitoring={{
        title: 'Monitorización',
        modeRowLabel: 'Perfil de muestreo',
        modeRowDescription: 'Bajo consumo mide cada 5 s; Normal cada segundo; Diagnóstico dos veces por segundo con detalle por núcleo.',
        modeLabel: 'Perfil de muestreo',
        mode: monitoringMode,
        modeOptions: [
          { value: 'low_power', label: 'Bajo consumo' },
          { value: 'normal', label: 'Normal' },
          { value: 'diagnostic', label: 'Diagnóstico' }
        ],
        onModeChange: (v) => {
          monitoringMode = v;
          samplingInterval = v === 'low_power' ? '5000' : v === 'diagnostic' ? '500' : '1000';
        },
        onBatteryRowLabel: 'En batería',
        onBatteryRowDescription: 'Qué hacer con el muestreo cuando el equipo funciona con batería.',
        onBatteryLabel: 'En batería',
        onBattery,
        onBatteryOptions: [
          { value: 'keep', label: 'Mantener' },
          { value: 'low_power', label: 'Bajo consumo' },
          { value: 'pause', label: 'Pausar' }
        ],
        onOnBatteryChange: (v) => (onBattery = v),
        advancedLabel: 'Avanzado',
        intervalRowLabel: 'Intervalo de muestreo',
        intervalRowDescription: 'Valor exacto del perfil activo.',
        intervalSelectLabel: 'Intervalo de muestreo',
        samplingInterval,
        samplingIntervalOptions: [
          { value: '500', label: '500 ms' },
          { value: '1000', label: '1 s' },
          { value: '2000', label: '2 s' },
          { value: '5000', label: '5 s' }
        ],
        onSamplingIntervalChange: (v) => (samplingInterval = v),
        intervalDisabled: monitoringMode !== 'normal',
        intervalDisabledReason: 'Los perfiles Bajo consumo y Diagnóstico fijan su propio intervalo.',
        perCoreHistoryLabel: 'Guardar detalle por núcleo en el historial',
        perCoreHistoryDescription: 'Multiplica el tamaño de la base de datos. En perfil Diagnóstico se guarda siempre.',
        perCoreHistory,
        onPerCoreHistoryChange: (v) => (perCoreHistory = v)
      }}
      tray={{
        title: 'Bandeja y notificaciones',
        backgroundLabel: 'Monitorización en segundo plano',
        backgroundDescription: 'Sigue leyendo sensores con la ventana cerrada. Desactivarla cambia el cierre a «Salir».',
        backgroundMonitoring,
        onBackgroundMonitoringChange: (v) => {
          backgroundMonitoring = v;
          if (!v) {
            startHiddenInTray = false;
            if (closeAction === 'tray') closeAction = 'exit';
          }
        },
        notificationsLabel: 'Notificaciones',
        notificationsDescription: 'Avisos térmicos, eléctricos, de colector y de prueba terminada. Apagadas de fábrica.',
        notifications,
        onNotificationsChange: (v) => {
          notifications = v;
          if (!v) notifyOnFinish = false;
        },
        testNotificationLabel: 'Probar notificación',
        onTestNotification: () => (lastAction = 'probar notificación'),
        testNotificationDisabled: !notifications,
        testNotificationDisabledReason: 'Activa las notificaciones para probarlas.',
        advancedLabel: 'Avanzado',
        quietPeriodLabel: 'Periodo de silencio',
        quietPeriodDescription: 'No se envían avisos entre las horas indicadas.',
        quietPeriodEnabled: quietEnabled,
        onQuietPeriodEnabledChange: (v) => (quietEnabled = v),
        quietPeriodStartLabel: 'Desde',
        quietPeriodEndLabel: 'Hasta',
        quietPeriodStart: quietStart,
        quietPeriodEnd: quietEnd,
        quietPeriodOptions: Array.from({ length: 24 }, (_, h) => ({ value: String(h), label: `${String(h).padStart(2, '0')}:00` })),
        onQuietPeriodChange: (s, e) => {
          quietStart = s;
          quietEnd = e;
        },
        alertRulesNote: 'Un aviso exige que la situación dure al menos 90 s y no se repite antes de 30 min (reglas v1, no configurables).'
      }}
      privacy={{
        title: 'Datos y privacidad',
        anonymizeExportsLabel: 'Anonimizar exportaciones',
        anonymizeExportsDescription: 'Elimina nombre del equipo, usuario, series, direcciones de red y rutas de los ficheros exportados. Nada sale del equipo por red.',
        anonymizeExports,
        onAnonymizeExportsChange: (v) => (anonymizeExports = v),
        retentionRowLabel: 'Retención de historial',
        retentionRowDescription: 'Cuánto tiempo se conservan las muestras antes de borrarse automáticamente.',
        retentionSelectLabel: 'Retención de historial',
        dataRetention,
        dataRetentionOptions: [
          { value: 'session', label: 'Solo esta sesión' },
          { value: '1d', label: '1 día' },
          { value: '7d', label: '7 días' },
          { value: '30d', label: '30 días' }
        ],
        onDataRetentionChange: (v) => (dataRetention = v),
        storageRowLabel: 'Espacio utilizado',
        storageValue: '38 MB',
        storageDescription: 'Base de datos 31 MB · Registros 7 MB · 12 sesiones · desde el 10 sept',
        corruptBackupRowLabel: 'Base de datos anterior dañada',
        corruptBackupNotice: undefined,
        corruptBackupExportLabel: 'Exportar',
        onExportCorruptBackup: () => (lastAction = 'exportar base de datos dañada'),
        exportRowLabel: 'Exportar',
        exportRowDescription: 'Abre el diálogo de exportación para la sesión activa.',
        exportButtonLabel: 'Exportar…',
        onExport: () => (lastAction = 'abrir ExportDialog (sesión activa)'),
        deleteRowLabel: 'Eliminar todos mis datos',
        deleteRowDescription: 'Borra sesiones, muestras, eventos, informes y referencias. Conserva idioma, tema, ajustes y ventana.',
        deleteButtonLabel: 'Eliminar…',
        deleteStatus,
        deleteErrorMessage: 'No se pudieron borrar los datos. Comprueba que ningún informe esté abierto e inténtalo de nuevo.',
        onDeleteAllData: simulateDelete,
        deleteDialogTitle: 'Eliminar todos mis datos',
        deleteDialogDescription:
          'Se borrarán todas las sesiones, muestras, eventos, informes y referencias guardados en este equipo. Las preferencias se conservan. Esta acción no se puede deshacer.',
        deleteDialogCancelLabel: 'Cancelar',
        deleteDialogConfirmLabel: 'Eliminar'
      }}
      sensors={{
        title: 'Sensores y cobertura',
        rowLabel: 'Cobertura de sensores',
        status: sensorStatus,
        detectingLabel: 'Detectando sensores…',
        coverageTitle: sensorStatus === 'complete' ? 'Cobertura completa' : 'Cobertura parcial',
        coverageDescription:
          sensorStatus === 'complete'
            ? 'Nivel A: temperatura, frecuencia activa y base, potencia, límites y razones de limitación.'
            : 'No se encontró el sensor de potencia; el diagnóstico eléctrico será indeterminado.',
        advancedAccess,
        advancedAccessNote: ACCESS_NOTES[advancedAccess],
        requestAccessLabel: 'Instalar acceso avanzado',
        onRequestAdvancedAccess: () => {
          advancedAccess = 'available';
          lastAction = 'instalar acceso avanzado (UAC)';
        },
        accessRetryLabel: 'Reintentar',
        onAccessRetry: () => {
          advancedAccess = 'available';
          lastAction = 'reintentar acceso avanzado';
        },
        recheckLabel: 'Volver a comprobar',
        onRecheck: () => {
          sensorStatus = 'detecting';
          lastAction = 'volver a comprobar sensores';
          setTimeout(() => (sensorStatus = 'complete'), 900);
        },
        coverage: showMatrix ? coverageSnippet : undefined
      }}
      diagnostics={{
        title: 'Diagnóstico',
        durationRowLabel: 'Duración de la carga sostenida',
        durationRowDescription: 'Corta 90 s · Estándar 3 min · Larga 5 min. Las demás fases no cambian.',
        durationLabel: 'Duración',
        duration,
        durationOptions: [
          { value: 'short', label: 'Corta' },
          { value: 'standard', label: 'Estándar' },
          { value: 'long', label: 'Larga' }
        ],
        onDurationChange: (v) => (duration = v),
        requireAcLabel: 'Exigir alimentación conectada',
        requireAcDescription: 'Si está apagado, en batería solo se advierte de posibles límites de potencia.',
        requireAc,
        onRequireAcChange: (v) => (requireAc = v),
        notifyLabel: 'Avisar al terminar',
        notifyDescription: 'Notifica cuando el diagnóstico guiado finaliza con la ventana en la bandeja.',
        notifyOnFinish,
        onNotifyOnFinishChange: (v) => (notifyOnFinish = v),
        notifyDisabled: !notifications,
        notifyDisabledReason: 'Requiere activar las notificaciones.',
        safetyLimitsTitle: 'Límites de seguridad (no configurables)',
        safetyLimits: [
          { label: 'Temperatura por encima del límite', value: '> límite + 2 °C durante 3 s' },
          { label: 'Refrigeración gravemente insuficiente', value: 'en el límite con < 50 % de la frecuencia base durante 10 s' },
          { label: 'Sensor crítico perdido', value: '3 muestras seguidas' },
          { label: 'Generador sin respuesta', value: '5 s' }
        ],
        safetyLimitsNote: 'Llegar al límite de temperatura no detiene la prueba: el procesador se protege solo y es lo que se mide. La prueba nunca modifica voltajes, potencia, ventiladores ni BIOS.'
      }}
      updates={{
        title: 'Actualizaciones',
        versionRowLabel: 'Versión actual',
        currentVersion: '1.4.2',
        autoCheckLabel: 'Buscar actualizaciones automáticamente',
        autoCheckDescription: 'Consulta GitHub Releases como máximo una vez cada 24 h. Apagado significa cero tráfico.',
        autoCheck,
        onAutoCheckChange: (v) => (autoCheck = v),
        lastCheckLabel: 'Última comprobación',
        lastCheckValue: autoCheck ? 'Hoy, 09:12' : 'Nunca',
        status: updateStatus,
        checkNowLabel: 'Buscar actualizaciones',
        onCheckNow: simulateCheckUpdate,
        checkingLabel: 'Buscando actualizaciones…',
        upToDateLabel: 'ThrottleWatch está al día.',
        availableVersion: 'Versión 1.5.0 disponible',
        availableDescription: 'Incluye mejoras en la detección de cobertura parcial de sensores. No se ha descargado nada todavía.',
        downloadLabel: 'Descargar',
        onDownload: simulateDownload,
        downloadPercent,
        downloadingLabel: 'Descargando actualización',
        verifiedLabel: 'Versión 1.5.0 descargada y verificada (Ed25519)',
        verifiedDescription: 'Instalar cerrará ThrottleWatch y lanzará el instalador.',
        installLabel: 'Instalar',
        onInstall: simulateInstall,
        installDisabled: installBlocked,
        installDisabledReason: 'Hay un diagnóstico guiado en curso. Espera a que termine o detenlo.',
        installingLabel: 'Instalando actualización…',
        errorMessage: 'No se pudo comprobar si hay actualizaciones. Revisa tu conexión e inténtalo de nuevo.',
        retryLabel: 'Reintentar',
        onRetry: simulateCheckUpdate
      }}
      about={{
        title: 'Acerca de y ayuda',
        versionRowLabel: 'Versión',
        version: '1.4.2 (build 20340)',
        buildLabel: 'Instalado el 3 de septiembre de 2026',
        repeatIntroLabel: 'Repetir introducción',
        repeatIntroDescription: 'Vuelve a mostrar las cinco diapositivas del primer inicio.',
        onRepeatIntro: () => (lastAction = 'repetir introducción'),
        links: [
          { label: 'Ayuda', onOpen: () => (lastAction = 'abrir ayuda empaquetada') },
          { label: 'Documentación en línea', external: true, externalHint: 'Se abre en el navegador del sistema.', onOpen: () => (lastAction = 'abrir documentación en línea') },
          { label: 'Licencias de terceros', onOpen: () => (lastAction = 'abrir licencias') },
          { label: 'Abrir carpeta de registros', onOpen: () => (lastAction = 'abrir carpeta de registros') }
        ],
        technicalSummary: technicalSummarySnippet,
        advancedLabel: 'Avanzado',
        detailedLoggingRowLabel: 'Registro detallado',
        detailedLoggingRowDescription: 'Se activa durante 24 horas y se desactiva al reiniciar la aplicación.',
        detailedLoggingLabel: 'Registro detallado',
        detailedLogging,
        detailedLoggingUntilLabel: detailedLogging ? 'Activo hasta el próximo reinicio o durante 24 horas.' : 'Desactivado',
        onDetailedLoggingChange: (checked) => (detailedLogging = checked)
      }}
      riskZone={{
        title: 'Zona de riesgo',
        resetRowLabel: 'Restablecer ThrottleWatch',
        resetRowDescription: 'Borra preferencias y datos, desregistra el inicio con Windows y elimina descargas pendientes. La próxima vez se repetirá el primer inicio.',
        resetButtonLabel: 'Restablecer…',
        resetStatus,
        resetErrorMessage: 'No se pudo restablecer ThrottleWatch. Inténtalo de nuevo.',
        onReset: simulateReset,
        resetDialogTitle: 'Restablecer ThrottleWatch',
        resetDialogDescription:
          'Se borrarán las preferencias y los datos guardados. La próxima vez que abras la app se repetirá el primer inicio. Esta acción no se puede deshacer.',
        resetDialogCancelLabel: 'Cancelar',
        resetDialogConfirmLabel: 'Restablecer'
      }}
    />
  </div>
</div>

<style>
  .settings-screen-demo {
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
    max-width: 760px;
    background: var(--surface);
    border: 1px solid var(--hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  /* This stacking is this DEMO's own layout responding to the browser
     window (it has to decide where to put its control panel) — not
     the SettingsScreen component, which never reads window size (see
     its own doc comment) and reflows purely via CSS container queries
     scoped to its own width. */
  @media (max-width: 900px) {
    .settings-screen-demo {
      flex-direction: column;
    }
    .control-panel {
      width: auto;
      position: static;
    }
  }
</style>
