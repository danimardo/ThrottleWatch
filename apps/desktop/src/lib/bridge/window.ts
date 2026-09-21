import {
  availableMonitors,
  getCurrentWindow,
  currentMonitor,
  LogicalPosition,
  LogicalSize,
  type Monitor
} from '@tauri-apps/api/window';
import { invokeValidated } from './index';
import {
  commandResponseSchemas,
  windowStateSchema,
  type WindowState
} from './schemas';
import {
  resolveGeometry,
  selectMonitor,
  type MonitorSnapshot,
  type WorkArea
} from '../../features/window/geometry';

export interface WindowAdapter {
  minimize(): Promise<void>;
  hide(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
  restoreSavedGeometry(): Promise<void>;
  persistGeometry(): Promise<void>;
  watchGeometryChanges(onChange: () => void): Promise<() => void>;
}

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

interface RuntimeMonitor extends MonitorSnapshot {
  monitor: Monitor;
  scaleFactor: number;
}

function toRuntimeMonitor(monitor: Monitor): RuntimeMonitor {
  const scaleFactor = monitor.scaleFactor > 0 ? monitor.scaleFactor : 1;
  return {
    monitor,
    scaleFactor,
    fingerprint: monitor.name,
    workArea: {
      x: monitor.workArea.position.x / scaleFactor,
      y: monitor.workArea.position.y / scaleFactor,
      width: monitor.workArea.size.width / scaleFactor,
      height: monitor.workArea.size.height / scaleFactor
    }
  };
}

function monitorKey(monitor: Monitor): string {
  return [
    monitor.name ?? 'unnamed',
    monitor.workArea.position.x,
    monitor.workArea.position.y,
    monitor.workArea.size.width,
    monitor.workArea.size.height,
    monitor.scaleFactor
  ].join(':');
}

function isRuntimeMonitor(
  monitor: MonitorSnapshot | null
): monitor is RuntimeMonitor {
  return monitor !== null && 'monitor' in monitor && 'scaleFactor' in monitor;
}

const browserWindowAdapter: WindowAdapter = {
  minimize: () => Promise.resolve(),
  hide: () => Promise.resolve(),
  toggleMaximize: () => Promise.resolve(),
  close: () => Promise.resolve(),
  isMaximized: () => Promise.resolve(false),
  restoreSavedGeometry: () => Promise.resolve(),
  persistGeometry: () => Promise.resolve(),
  watchGeometryChanges: () => Promise.resolve(() => undefined)
};

export function createWindowAdapter(): WindowAdapter {
  if (!hasTauriRuntime()) return browserWindowAdapter;
  const current = getCurrentWindow();
  let lastMonitorKey: string | null = null;

  async function savedMonitor(
    saved: WindowState
  ): Promise<RuntimeMonitor | null> {
    const [currentMonitorValue, monitors] = await Promise.all([
      currentMonitor(),
      availableMonitors().catch(() => [])
    ]);
    const currentSnapshot = currentMonitorValue
      ? toRuntimeMonitor(currentMonitorValue)
      : null;
    const snapshots = monitors.map(toRuntimeMonitor);
    const selected = selectMonitor(saved, snapshots, currentSnapshot);
    return isRuntimeMonitor(selected) ? selected : currentSnapshot;
  }

  async function refreshMinimumSize(): Promise<void> {
    const monitor = await currentMonitor();
    if (!monitor) return;
    const key = monitorKey(monitor);
    if (key === lastMonitorKey) return;
    const runtimeMonitor = toRuntimeMonitor(monitor);
    const geometry = resolveGeometry(null, runtimeMonitor.workArea);
    await current.setMinSize(
      new LogicalSize(geometry.min_width, geometry.min_height)
    );
    lastMonitorKey = key;
  }

  async function restoreSavedGeometry(): Promise<void> {
    const result = await invokeValidated(
      'get_window_state',
      undefined,
      commandResponseSchemas.get_window_state
    );
    if (!result.ok) return;
    const monitor = await savedMonitor(result.value);
    const workArea: WorkArea = monitor?.workArea ?? {
      x: 0,
      y: 0,
      width: 1920,
      height: 1080
    };
    const geometry = resolveGeometry(result.value, workArea);
    await current.setMinSize(
      new LogicalSize(geometry.min_width, geometry.min_height)
    );
    await current.setSize(
      new LogicalSize(geometry.restored_width, geometry.restored_height)
    );
    await current.setPosition(
      new LogicalPosition(geometry.restored_x, geometry.restored_y)
    );
    if ((await current.isMaximized()) !== geometry.maximized) {
      await current.toggleMaximize();
    }
    if (monitor) lastMonitorKey = monitorKey(monitor.monitor);
  }

  return {
    minimize: () => current.minimize(),
    hide: () => current.hide(),
    toggleMaximize: () => current.toggleMaximize(),
    close: () => current.close(),
    isMaximized: () => current.isMaximized(),
    restoreSavedGeometry,
    persistGeometry: async () => {
      const previous = await invokeValidated(
        'get_window_state',
        undefined,
        commandResponseSchemas.get_window_state
      );
      if (!previous.ok) return;
      const maximized = await current.isMaximized();
      const monitor = await currentMonitor();
      const runtimeMonitor = monitor ? toRuntimeMonitor(monitor) : null;
      const scale =
        runtimeMonitor?.scaleFactor ?? (await current.scaleFactor());
      const position = await current.innerPosition();
      const size = await current.innerSize();
      const next: WindowState = maximized
        ? {
            ...previous.value,
            maximized: true,
            display_fingerprint:
              runtimeMonitor?.fingerprint ?? previous.value.display_fingerprint,
            updated_at: new Date().toISOString()
          }
        : {
            ...previous.value,
            restored_x: Math.round(position.x / scale),
            restored_y: Math.round(position.y / scale),
            restored_width: Math.max(480, Math.round(size.width / scale)),
            restored_height: Math.max(500, Math.round(size.height / scale)),
            maximized: false,
            display_fingerprint: runtimeMonitor?.fingerprint ?? null,
            updated_at: new Date().toISOString()
          };
      await invokeValidated(
        'set_window_state',
        { request: next },
        windowStateSchema
      );
    },
    watchGeometryChanges: async (onChange) => {
      const unlistenMove = await current.listen('tauri://move', () => {
        void refreshMinimumSize().then(onChange);
      });
      const unlistenResize = await current.listen('tauri://resize', onChange);
      const unlistenScale = await current.onScaleChanged(() => {
        void restoreSavedGeometry().then(onChange);
      });
      return () => {
        unlistenMove();
        unlistenResize();
        unlistenScale();
      };
    }
  };
}
