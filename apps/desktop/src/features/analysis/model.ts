import type { AnalysisWindow } from '../../lib/bridge/schemas';

export type AnalysisTrackKind = 'temperature' | 'clock' | 'load' | 'power';
export type AnalysisEventKind =
  'thermal' | 'electrical' | 'mixed' | 'platform' | 'info';
export type AnalysisTone =
  'accent' | 'thermal' | 'warm' | 'normal' | 'power' | 'unknown';
export interface AnalysisPoint {
  t: number;
  value: number | null;
  reducedQuality?: boolean;
}
export interface AnalysisTrack {
  kind: AnalysisTrackKind;
  label: string;
  tone: AnalysisTone;
  points: AnalysisPoint[];
  min: number;
  max: number;
}
export interface AnalysisEvent {
  id: string;
  kind: AnalysisEventKind;
  startT: number;
  endT: number;
  label: string;
}

const kinds: Record<string, AnalysisTrackKind> = {
  temperature: 'temperature',
  clock: 'clock',
  load: 'load',
  power: 'power'
};
const tones: Record<AnalysisTrackKind, AnalysisTone> = {
  temperature: 'warm',
  clock: 'accent',
  load: 'normal',
  power: 'power'
};

export function toAnalysisView(window: AnalysisWindow): {
  tracks: AnalysisTrack[];
  events: AnalysisEvent[];
} {
  const tracks = window.tracks.reduce<AnalysisTrack[]>((result, track) => {
    const kind = kinds[track.kind];
    if (!kind) return result;
    const points: AnalysisPoint[] = track.points.map((point) => ({
      t: Math.round((point.start_ms + point.end_ms) / 2),
      value: point.average,
      reducedQuality: point.quality !== 'complete' || point.gap
    }));
    const values = points.reduce<number[]>(
      (valuesSoFar, point) =>
        point.value === null ? valuesSoFar : [...valuesSoFar, point.value],
      []
    );
    result.push({
      kind,
      label: track.kind,
      tone: tones[kind],
      points,
      min: Math.min(...values, 0),
      max: Math.max(...values, 1)
    });
    return result;
  }, []);
  const events = window.events.reduce<AnalysisEvent[]>((result, event) => {
    const kind =
      event.kind === 'thermal' ||
      event.kind === 'electrical' ||
      event.kind === 'mixed' ||
      event.kind === 'platform'
        ? event.kind
        : 'info';
    result.push({
      id: event.id,
      kind,
      startT: event.start_ms,
      endT: event.end_ms,
      label: event.label
    });
    return result;
  }, []);
  return { tracks, events };
}
