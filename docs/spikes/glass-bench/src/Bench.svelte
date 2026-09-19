<script lang="ts">
  // T019c bench: the app shell as the design system prescribes it (ambient
  // background, glass sidebar, glass title bar, a floating glass panel) with
  // the real AnalysisChart underneath, receiving one new sample per interval.
  // Query parameters: glass=full|reduced|off, points=N, interval=ms (0 = idle),
  // seconds=N (measurement window), delay=ms (warm-up before measuring). Results land in window.__bench and #result.
  import { onMount } from 'svelte';
  import AnalysisChart from '../../../../design/components/AnalysisChart.svelte';
  import type { AnalysisTrack, AnalysisPoint } from '../../../../design/components/AnalysisChart.svelte';
  import { applyGlassLevel, type GlassLevel } from '../../../../design/tokens/tokens';

  const params = new URLSearchParams(location.search);
  const glass = (params.get('glass') ?? 'full') as GlassLevel;
  const pointCount = Number(params.get('points') ?? 3000);
  const interval = Number(params.get('interval') ?? 1000);
  const seconds = Number(params.get('seconds') ?? 20);
  // Measurement starts after `delay` ms so the load transient is excluded and the
  // window lines up with the host's CPU window (host waits 2.5 s after navigating).
  const delay = Number(params.get('delay') ?? 3000);
  // ambient=0 freezes the drifting background; chart=0 removes the chart;
  // measure=0 skips the requestAnimationFrame counter (pure idle CPU measurement:
  // the page then publishes an empty result after `seconds`).
  const ambient = params.get('ambient') !== '0';
  const chart = params.get('chart') !== '0';
  const measure = params.get('measure') !== '0';

  applyGlassLevel(glass);

  function series(base: number, amplitude: number, min: number, max: number): AnalysisPoint[] {
    const points: AnalysisPoint[] = [];
    for (let i = 0; i < pointCount; i += 1) {
      const value = base + amplitude * Math.sin(i / 37) + (Math.random() - 0.5) * amplitude * 0.3;
      points.push({ t: i, value: Math.min(max, Math.max(min, value)) });
    }
    return points;
  }

  let tracks = $state<AnalysisTrack[]>([
    { kind: 'temperature', label: 'Temperatura', unit: '°C', tone: 'thermal', points: series(82, 12, 30, 100), min: 30, max: 100 },
    { kind: 'clock', label: 'Reloj activo', unit: 'MHz', tone: 'accent', points: series(4200, 600, 800, 5200), min: 800, max: 5200 },
    { kind: 'power', label: 'Potencia', unit: 'W', tone: 'power', points: series(38, 9, 0, 65), min: 0, max: 65 },
    { kind: 'load', label: 'Carga', unit: '%', tone: 'normal', points: series(70, 25, 0, 100), min: 0, max: 100 }
  ] as AnalysisTrack[]);

  let frames = $state<number[]>([]);
  let result = $state('');

  function pushSample() {
    tracks = tracks.map((track) => {
      const last = track.points[track.points.length - 1];
      const next = {
        t: last.t + 1,
        value: Math.min(track.max, Math.max(track.min, (last.value ?? track.min) + (Math.random() - 0.5) * (track.max - track.min) * 0.05))
      };
      return { ...track, points: [...track.points.slice(1), next] };
    });
  }

  onMount(() => {
    let sampler: ReturnType<typeof setInterval> | undefined;
    if (interval > 0) sampler = setInterval(pushSample, interval);

    const perSecond: number[] = [];
    let count = 0;
    let windowStart = performance.now();
    let started = windowStart;
    let raf = 0;
    const tick = (now: number) => {
      count += 1;
      if (now - windowStart >= 1000) {
        perSecond.push(count);
        count = 0;
        windowStart = now;
        frames = [...perSecond];
      }
      if (now - started < seconds * 1000) {
        raf = requestAnimationFrame(tick);
      } else {
        const sorted = [...perSecond].sort((a, b) => a - b);
        const summary = {
          glass,
          points: pointCount,
          interval_ms: interval,
          seconds: perSecond.length,
          fps_min: sorted[0],
          fps_p10: sorted[Math.floor(sorted.length * 0.1)],
          fps_median: sorted[Math.floor(sorted.length / 2)],
          fps_mean: Math.round((perSecond.reduce((a, b) => a + b, 0) / perSecond.length) * 10) / 10,
          ambient,
          chart,
          backdrop_supported: CSS.supports('backdrop-filter', 'blur(1px)'),
          ua: navigator.userAgent
        };
        (window as unknown as { __bench: unknown }).__bench = summary;
        result = JSON.stringify(summary);
        if (sampler) clearInterval(sampler);
      }
    };
    const kickoff = setTimeout(() => {
      windowStart = performance.now();
      started = windowStart;
      if (measure) {
        raf = requestAnimationFrame(tick);
      } else {
        setTimeout(() => {
          const summary = { glass, points: pointCount, interval_ms: interval, seconds, measure: false, ambient, chart };
          (window as unknown as { __bench: unknown }).__bench = summary;
          result = JSON.stringify(summary);
        }, seconds * 1000);
      }
    }, delay);
    return () => {
      clearTimeout(kickoff);
      cancelAnimationFrame(raf);
      if (sampler) clearInterval(sampler);
    };
  });
</script>

<div class="shell" class:tw-ambient={ambient} class:static-bg={!ambient}>
  <header class="titlebar tw-glass-strong">ThrottleWatch — banco T019c · vidrio {glass} · {pointCount} puntos · {interval} ms</header>
  <aside class="sidebar tw-glass">
    <nav>
      {#each ['Ahora', 'Análisis', 'CPU', 'Sesiones', 'Ajustes'] as item}
        <div class="item">{item}</div>
      {/each}
    </nav>
    <div class="fps">fps: {frames.slice(-5).join(' ')}</div>
  </aside>
  <main class="content">
    <section class="card tw-glass">
      {#if chart}
      <AnalysisChart
        {tracks}
        valueLabel={(point, track) => (point?.value == null ? 'sin datos' : `${Math.round(point.value)} ${track.unit ?? ''}`)}
        timeLabel={(t) => `${t} s`}
        legendLabel="Pistas"
        cursorLabel="Cursor"
        rangeSummaryLabel={(range) => `Muestras ${range[0]}–${range[1]}`}
        resetRangeLabel="Quitar selección"
        trackHeight={96}
      />
      {/if}
    </section>
    <div class="floating tw-glass-strong">Panel flotante sobre el gráfico (peor caso del desenfoque)</div>
  </main>
  <pre id="result">{result}</pre>
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    grid-template-rows: 40px 1fr;
    height: 100vh;
    color: var(--text);
    font-family: var(--font-ui, system-ui, sans-serif);
  }
  .static-bg {
    background-color: var(--bg);
    background-image:
      radial-gradient(70% 60% at 10% 5%, var(--ambient-1), transparent 68%),
      radial-gradient(65% 55% at 92% 95%, var(--ambient-2), transparent 68%);
  }
  .titlebar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 13px;
    position: relative;
    z-index: 2;
  }
  .sidebar {
    padding: 12px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    position: relative;
    z-index: 1;
  }
  .item {
    padding: 8px 10px;
    border-radius: 8px;
  }
  .fps {
    font-size: 12px;
    opacity: 0.8;
  }
  .content {
    position: relative;
    padding: 16px;
    overflow: hidden;
  }
  .card {
    border-radius: 14px;
    padding: 12px;
  }
  .floating {
    position: absolute;
    right: 40px;
    top: 140px;
    width: 320px;
    padding: 18px;
    border-radius: 14px;
    z-index: 3;
  }
  #result {
    position: fixed;
    left: 0;
    bottom: 0;
    margin: 0;
    font-size: 10px;
    opacity: 0.001;
  }
</style>
