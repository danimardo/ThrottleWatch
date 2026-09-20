import { describe, expect, it } from 'vitest';
import { toAnalysisView } from './model';

describe('analysis model', () => {
  it('keeps gaps as reduced points and maps informational events', () => {
    const view = toAnalysisView({
      session_id: 's1',
      start_ms: 0,
      end_ms: 100,
      is_aggregated: true,
      tracks: [
        {
          kind: 'temperature',
          points: [
            {
              start_ms: 0,
              end_ms: 50,
              first: 40,
              last: 50,
              min: 40,
              max: 50,
              average: 45,
              quality: 'complete',
              gap: false
            },
            {
              start_ms: 50,
              end_ms: 100,
              first: null,
              last: null,
              min: null,
              max: null,
              average: null,
              quality: 'missing',
              gap: true
            }
          ]
        }
      ],
      events: [
        {
          id: 'e1',
          kind: 'turbo_end',
          start_ms: 40,
          end_ms: 40,
          label: 'Fin de turbo',
          informational: true
        }
      ]
    });
    const track = view.tracks[0];
    const event = view.events[0];
    expect(track).toBeDefined();
    expect(event).toBeDefined();
    if (!track || !event) return;
    expect(track.points[1]?.value).toBeNull();
    expect(track.points[1]?.reducedQuality).toBe(true);
    expect(event.kind).toBe('info');
  });

  it('keeps four dense tracks within the frontend rendering budget', () => {
    const points = Array.from({ length: 3000 }, (_, index) => ({
      start_ms: index * 100,
      end_ms: (index + 1) * 100,
      first: index,
      last: index + 1,
      min: index,
      max: index + 1,
      average: index + 0.5,
      quality: 'complete' as const,
      gap: false
    }));
    const started = performance.now();
    const view = toAnalysisView({
      session_id: 'dense',
      start_ms: 0,
      end_ms: 300_000,
      is_aggregated: true,
      tracks: ['temperature', 'clock', 'load', 'power'].map((kind) => ({
        kind,
        points
      })),
      events: []
    });

    expect(view.tracks).toHaveLength(4);
    expect(view.tracks.every((track) => track.points.length === 3000)).toBe(
      true
    );
    expect(performance.now() - started).toBeLessThan(250);
  });
});
