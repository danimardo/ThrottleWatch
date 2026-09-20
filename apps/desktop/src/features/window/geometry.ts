export interface WorkArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SavedGeometry {
  restored_x: number;
  restored_y: number;
  restored_width: number;
  restored_height: number;
  maximized: boolean;
  display_fingerprint?: string | null;
}

export interface MonitorSnapshot {
  fingerprint: string | null;
  workArea: WorkArea;
}

export interface ResolvedGeometry extends SavedGeometry {
  min_width: number;
  min_height: number;
}

export const DEFAULT_WIDTH = 1100;
export const DEFAULT_HEIGHT = 760;
export const MIN_WIDTH = 480;
export const MIN_HEIGHT = 600;
export const FALLBACK_MIN_HEIGHT = 500;

export function minimumHeightFor(workAreaHeight: number): number {
  return workAreaHeight < MIN_HEIGHT ? FALLBACK_MIN_HEIGHT : MIN_HEIGHT;
}

export function isUsablyVisible(geometry: SavedGeometry, workArea: WorkArea): boolean {
  const visibleWidth = Math.min(geometry.restored_x + geometry.restored_width, workArea.x + workArea.width) -
    Math.max(geometry.restored_x, workArea.x);
  const visibleHeight = Math.min(geometry.restored_y + geometry.restored_height, workArea.y + workArea.height) -
    Math.max(geometry.restored_y, workArea.y);
  return visibleWidth >= 80 && visibleHeight >= 40;
}

export function selectMonitor(
  saved: SavedGeometry | null,
  monitors: readonly MonitorSnapshot[],
  current: MonitorSnapshot | null
): MonitorSnapshot | null {
  if (monitors.length === 0) return current;

  const savedFingerprint = saved?.display_fingerprint;
  if (savedFingerprint) {
    const fingerprintMatch = monitors.find(
      (monitor) => monitor.fingerprint === savedFingerprint
    );
    if (fingerprintMatch) return fingerprintMatch;
  }

  if (saved) {
    const visibleMatch = monitors.find((monitor) =>
      isUsablyVisible(saved, monitor.workArea)
    );
    if (visibleMatch) return visibleMatch;
  }

  if (!current) return monitors[0] ?? null;
  const currentMatch = monitors.find(
    (monitor) =>
      monitor.fingerprint === current.fingerprint &&
      monitor.workArea.x === current.workArea.x &&
      monitor.workArea.y === current.workArea.y
  );
  return currentMatch ?? current;
}

export function resolveGeometry(
  saved: SavedGeometry | null,
  workArea: WorkArea
): ResolvedGeometry {
  const minHeight = minimumHeightFor(workArea.height);
  const tooShortToRestore = workArea.height < FALLBACK_MIN_HEIGHT;
  const minWidth = Math.min(MIN_WIDTH, workArea.width);
  const width = Math.min(
    Math.max(saved?.restored_width ?? DEFAULT_WIDTH, minWidth),
    workArea.width
  );
  const height = Math.min(
    Math.max(saved?.restored_height ?? DEFAULT_HEIGHT, minHeight),
    workArea.height
  );
  const visible = saved !== null && isUsablyVisible(saved, workArea);
  return {
    restored_x: visible ? saved.restored_x : workArea.x + Math.round((workArea.width - width) / 2),
    restored_y: visible ? saved.restored_y : workArea.y + Math.round((workArea.height - height) / 2),
    restored_width: width,
    restored_height: height,
    maximized: tooShortToRestore || saved?.maximized === true,
    min_width: minWidth,
    min_height: minHeight
  };
}
