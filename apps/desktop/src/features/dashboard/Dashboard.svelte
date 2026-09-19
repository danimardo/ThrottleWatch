<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ContextStrip,
    CoverageMatrix,
    StatWidget,
    StatusHero
  } from '../../design-system/components';
  import {
    invokeValidated,
    listenValidated,
    parseEvent
  } from '../../lib/bridge';
  import {
    coverageMatrixSchema,
    liveSnapshotSchema,
    commandResponseSchemas
  } from '../../lib/bridge/schemas';
  import { createTranslator } from '../../lib/i18n';
  import { toStatusHeroView } from './adapter';
  import {
    createDemoSnapshot,
    formatNumber,
    fromLiveSnapshot,
    marginText,
    type DashboardSnapshot
  } from './model';

  const { t, locale } = createTranslator(
    'system',
    typeof navigator === 'undefined' ? 'en-US' : navigator.language
  );
  const numberLocale = locale === 'es' ? 'es-ES' : 'en-US';

  function localizedCollectorLabel(value: string): string {
    const keys: Record<string, string> = {
      running: 'dashboard.collectorRunning',
      degraded: 'dashboard.collectorDegraded',
      restarting: 'dashboard.collectorRestarting',
      stopped: 'dashboard.collectorStopped',
      failed: 'dashboard.collectorFailed'
    };
    return keys[value] === undefined ? value : t(keys[value]);
  }

  interface Props {
    snapshot?: DashboardSnapshot;
  }

  let { snapshot: incomingSnapshot }: Props = $props();
  let currentSnapshot = $state<DashboardSnapshot>(createDemoSnapshot(t));
  let hero = $derived(
    toStatusHeroView(
      {
        ...currentSnapshot,
        collectorLabel: localizedCollectorLabel(currentSnapshot.collectorLabel)
      },
      t,
      numberLocale
    )
  );
  let coverageOpen = $state(false);
  let coverageRows = $state<
    Array<{
      id: string;
      label: string;
      available: boolean;
      quality: 'direct' | 'derived' | 'substitute';
      qualityLabel: string;
      sourceLabel: string;
      reasonLabel: string;
    }>
  >([]);
  let advancedAccess = $state<DashboardSnapshot['advancedAccess']>(
    createDemoSnapshot(t).advancedAccess
  );

  $effect(() => {
    currentSnapshot = incomingSnapshot ?? createDemoSnapshot(t);
  });

  onMount(() => {
    let disposed = false;
    const unlisten: Array<() => void> = [];
    void invokeValidated(
      'get_live_snapshot',
      undefined,
      liveSnapshotSchema
    ).then((result) => {
      if (!disposed && result.ok)
        currentSnapshot = fromLiveSnapshot(result.value);
      if (!disposed && result.ok)
        advancedAccess = result.value.coverage.advanced_access;
    });
    void listenValidated('telemetry:snapshot', (value) => {
      const parsed = parseEvent('telemetry:snapshot', value);
      if (!disposed && parsed.ok)
        currentSnapshot = fromLiveSnapshot(parsed.value);
      if (!disposed && parsed.ok)
        advancedAccess = parsed.value.coverage.advanced_access;
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    void listenValidated('collector:state', (value) => {
      if (disposed) return;
      const parsed = parseEvent('collector:state', value);
      if (!parsed.ok) return;
      currentSnapshot = {
        ...currentSnapshot,
        collectorState:
          parsed.value.state === 'running'
            ? 'fresh'
            : parsed.value.state === 'degraded' ||
                parsed.value.state === 'restarting'
              ? 'stale'
              : 'disconnected',
        collectorLabel: parsed.value.state
      };
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    void listenValidated('coverage:changed', (value) => {
      if (disposed) return;
      const parsed = parseEvent('coverage:changed', value);
      if (!parsed.ok) return;
      currentSnapshot = {
        ...currentSnapshot,
        coverage: parsed.value.tier,
        confidenceLabel: `Maximum reachable confidence: ${parsed.value.confidence_ceiling}`
      };
      advancedAccess = parsed.value.advanced_access;
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    void listenValidated('power:context', (value) => {
      if (disposed) return;
      const parsed = parseEvent('power:context', value);
      if (!parsed.ok) return;
      currentSnapshot = {
        ...currentSnapshot,
        powerLabel:
          parsed.value.source === 'battery'
            ? `Battery${parsed.value.battery_percent === undefined || parsed.value.battery_percent === null ? '' : ` ${String(parsed.value.battery_percent)} %`}`
            : parsed.value.source
      };
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    return () => {
      disposed = true;
      unlisten.forEach((stop) => stop());
    };
  });

  $effect(() => {
    if (!coverageOpen) return;
    void invokeValidated('get_coverage', undefined, coverageMatrixSchema).then(
      (result) => {
        if (!result.ok) return;
        coverageRows = result.value.rows.map((row) => ({
          id: row.id,
          label: row.label,
          available: row.available,
          quality:
            row.quality === 'derived' || row.quality === 'substitute'
              ? row.quality
              : 'direct',
          qualityLabel: row.quality_label ?? 'Unknown',
          sourceLabel: row.source_label ?? 'Unavailable',
          reasonLabel: row.reason_key ?? 'coverage.unavailable'
        }));
        advancedAccess = result.value.advanced_access;
      }
    );
  });

  function recheckCoverage() {
    void invokeValidated(
      'recheck_coverage',
      undefined,
      coverageMatrixSchema
    ).then((result) => {
      if (!result.ok) return;
      coverageRows = result.value.rows.map((row) => ({
        id: row.id,
        label: row.label,
        available: row.available,
        quality:
          row.quality === 'derived' || row.quality === 'substitute'
            ? row.quality
            : 'direct',
        qualityLabel: row.quality_label ?? 'Unknown',
        sourceLabel: row.source_label ?? 'Unavailable',
        reasonLabel: row.reason_key ?? 'coverage.unavailable'
      }));
      advancedAccess = result.value.advanced_access;
    });
  }

  function requestAdvancedAccess() {
    void invokeValidated(
      'request_low_level_access',
      {
        request: {
          action: advancedAccess === 'installable' ? 'install' : 'repair'
        }
      },
      commandResponseSchemas.request_low_level_access
    ).then((result) => {
      if (result.ok) recheckCoverage();
    });
  }

  function disableAdvancedAccess() {
    void invokeValidated(
      'disable_advanced_access',
      undefined,
      coverageMatrixSchema
    ).then((result) => {
      if (!result.ok) return;
      coverageRows = result.value.rows.map((row) => ({
        id: row.id,
        label: row.label,
        available: row.available,
        quality:
          row.quality === 'derived' || row.quality === 'substitute'
            ? row.quality
            : 'direct',
        qualityLabel: row.quality_label ?? 'Unknown',
        sourceLabel: row.source_label ?? 'Unavailable',
        reasonLabel: row.reason_key ?? 'coverage.unavailable'
      }));
      advancedAccess = result.value.advanced_access;
    });
  }
</script>

<main class="dashboard">
  <ContextStrip
    cpuLabel={currentSnapshot.cpuLabel}
    topologyLabel={currentSnapshot.topologyLabel}
    powerLabel={currentSnapshot.powerLabel}
    collectorState={currentSnapshot.collectorState}
    collectorLabel={localizedCollectorLabel(currentSnapshot.collectorLabel)}
    coverageActionLabel={t('dashboard.viewCoverage')}
    onCoverage={() => (coverageOpen = !coverageOpen)}
  />
  <StatusHero {...hero} />
  <section class="stats" aria-label={t('dashboard.currentSignals')}>
    {#snippet thermometerIcon()}<span aria-hidden="true">°</span>{/snippet}
    {#snippet loadIcon()}<span aria-hidden="true">▾</span>{/snippet}
    {#snippet clockIcon()}<span aria-hidden="true">◷</span>{/snippet}
    {#snippet powerIcon()}<span aria-hidden="true">ϟ</span>{/snippet}
    <StatWidget
      icon={thermometerIcon}
      tone={currentSnapshot.temperatureC === null ? 'unknown' : 'thermal'}
      label={t('dashboard.temperature')}
      value={formatNumber(
        currentSnapshot.temperatureC,
        0,
        numberLocale,
        t('dashboard.unavailableShort')
      )}
      unit={currentSnapshot.temperatureC === null ? undefined : '°C'}
      footnote={marginText(currentSnapshot, t, numberLocale)}
      enterIndex={0}
    />
    <StatWidget
      icon={loadIcon}
      tone="accent"
      label={t('dashboard.load')}
      value={formatNumber(
        currentSnapshot.loadPercent,
        0,
        numberLocale,
        t('dashboard.unavailableShort')
      )}
      unit={currentSnapshot.loadPercent === null ? undefined : '%'}
      footnote={t('dashboard.activeProcessors')}
      enterIndex={1}
    />
    <StatWidget
      icon={clockIcon}
      tone="accent"
      label={t('dashboard.activeClock')}
      value={formatNumber(
        currentSnapshot.activeClockMhz,
        0,
        numberLocale,
        t('dashboard.unavailableShort')
      )}
      unit={currentSnapshot.activeClockMhz === null ? undefined : 'MHz'}
      footnote={currentSnapshot.baseClockMhz === null
        ? t('dashboard.baseClockUnavailable')
        : `${t('dashboard.base')} ${formatNumber(currentSnapshot.baseClockMhz, 0, numberLocale, t('dashboard.unavailableShort'))} MHz`}
      enterIndex={2}
    />
    <StatWidget
      icon={powerIcon}
      tone={currentSnapshot.powerLimitW !== null ? 'power' : 'unknown'}
      label={t('dashboard.packagePower')}
      value={formatNumber(
        currentSnapshot.packagePowerW,
        0,
        numberLocale,
        t('dashboard.unavailableShort')
      )}
      unit={currentSnapshot.packagePowerW === null ? undefined : 'W'}
      footnote={currentSnapshot.powerLimitW === null
        ? t('dashboard.powerLimitUnavailable')
        : `${t('dashboard.limit')} ${formatNumber(currentSnapshot.powerLimitW, 0, numberLocale, t('dashboard.unavailableShort'))} W`}
      enterIndex={3}
    />
  </section>
  {#if coverageOpen}
    <section class="coverage" aria-label={t('dashboard.equipmentCoverage')}>
      <CoverageMatrix
        title={t('dashboard.equipmentCoverage')}
        rows={coverageRows}
        columnLabels={{
          magnitude: t('dashboard.magnitude'),
          available: t('dashboard.available'),
          quality: t('dashboard.quality'),
          source: t('dashboard.source'),
          reason: t('dashboard.reason')
        }}
        availableLabel={t('dashboard.available')}
        unavailableLabel={t('dashboard.unavailable')}
        maxConfidenceLabel={currentSnapshot.confidenceLabel}
        accessState={advancedAccess}
        accessLabel={advancedAccess === 'not_needed'
          ? t('dashboard.advancedNotNeeded')
          : t('dashboard.advancedCanImprove')}
        requestAccessLabel={t('dashboard.installAdvanced')}
        onRequestAccess={requestAdvancedAccess}
        accessRetryLabel={t('dashboard.repairAdvanced')}
        onAccessRetry={requestAdvancedAccess}
        disableAccessLabel={t('dashboard.disableAdvanced')}
        onDisableAccess={disableAdvancedAccess}
        recheckLabel={t('dashboard.checkAgain')}
        onRecheck={recheckCoverage}
      />
    </section>
  {/if}
</main>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 1100px;
    margin: 0 auto;
    padding: var(--space-6);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--space-3);
  }
  .coverage {
    padding-top: var(--space-2);
  }
</style>
