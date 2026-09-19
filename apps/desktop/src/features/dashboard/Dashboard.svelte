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
  import { toStatusHeroView } from './adapter';
  import {
    DEMO_SNAPSHOT,
    formatNumber,
    fromLiveSnapshot,
    marginText,
    type DashboardSnapshot
  } from './model';

  interface Props {
    snapshot?: DashboardSnapshot;
  }

  let { snapshot: incomingSnapshot }: Props = $props();
  let currentSnapshot = $state<DashboardSnapshot>(DEMO_SNAPSHOT);
  let hero = $derived(toStatusHeroView(currentSnapshot));
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
    DEMO_SNAPSHOT.advancedAccess
  );

  $effect(() => {
    currentSnapshot = incomingSnapshot ?? DEMO_SNAPSHOT;
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
    collectorLabel={currentSnapshot.collectorLabel}
    coverageActionLabel="View coverage"
    onCoverage={() => (coverageOpen = !coverageOpen)}
  />
  <StatusHero {...hero} />
  <section class="stats" aria-label="Current signals">
    {#snippet thermometerIcon()}<span aria-hidden="true">°</span>{/snippet}
    {#snippet loadIcon()}<span aria-hidden="true">▾</span>{/snippet}
    {#snippet clockIcon()}<span aria-hidden="true">◷</span>{/snippet}
    {#snippet powerIcon()}<span aria-hidden="true">ϟ</span>{/snippet}
    <StatWidget
      icon={thermometerIcon}
      tone={currentSnapshot.temperatureC === null ? 'unknown' : 'thermal'}
      label="Temperature"
      value={formatNumber(currentSnapshot.temperatureC)}
      unit={currentSnapshot.temperatureC === null ? undefined : '°C'}
      footnote={marginText(currentSnapshot)}
      enterIndex={0}
    />
    <StatWidget
      icon={loadIcon}
      tone="accent"
      label="Load"
      value={formatNumber(currentSnapshot.loadPercent)}
      unit={currentSnapshot.loadPercent === null ? undefined : '%'}
      footnote="Active processors in sample"
      enterIndex={1}
    />
    <StatWidget
      icon={clockIcon}
      tone="accent"
      label="Active clock"
      value={formatNumber(currentSnapshot.activeClockMhz)}
      unit={currentSnapshot.activeClockMhz === null ? undefined : 'MHz'}
      footnote={currentSnapshot.baseClockMhz === null
        ? 'Base clock unavailable'
        : `Base ${formatNumber(currentSnapshot.baseClockMhz)} MHz`}
      enterIndex={2}
    />
    <StatWidget
      icon={powerIcon}
      tone={currentSnapshot.powerLimitW !== null ? 'power' : 'unknown'}
      label="Package power"
      value={formatNumber(currentSnapshot.packagePowerW)}
      unit={currentSnapshot.packagePowerW === null ? undefined : 'W'}
      footnote={currentSnapshot.powerLimitW === null
        ? 'Power limit unavailable'
        : `Limit ${formatNumber(currentSnapshot.powerLimitW)} W`}
      enterIndex={3}
    />
  </section>
  {#if coverageOpen}
    <section class="coverage" aria-label="Sensor coverage">
      <CoverageMatrix
        title="Equipment coverage"
        rows={coverageRows}
        columnLabels={{
          magnitude: 'Magnitude',
          available: 'Available',
          quality: 'Quality',
          source: 'Source',
          reason: 'Reason'
        }}
        availableLabel="Available"
        unavailableLabel="Unavailable"
        maxConfidenceLabel={currentSnapshot.confidenceLabel}
        accessState={advancedAccess}
        accessLabel={advancedAccess === 'not_needed'
          ? 'Advanced access is not needed.'
          : 'Advanced access can improve coverage.'}
        requestAccessLabel="Install advanced access"
        onRequestAccess={requestAdvancedAccess}
        accessRetryLabel="Repair advanced access"
        onAccessRetry={requestAdvancedAccess}
        disableAccessLabel="Disable advanced access"
        onDisableAccess={disableAdvancedAccess}
        recheckLabel="Check again"
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
