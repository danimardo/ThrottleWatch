<script lang="ts">
  import { AnalysisChart, type AnalysisTrack } from '../../../components';

  const tracks: AnalysisTrack[] = [
    {
      kind: 'temperature',
      label: 'Temperatura',
      tone: 'thermal',
      points: [
        { t: 0, value: 60 },
        { t: 1, value: 62 },
        { t: 2, value: 65 },
        { t: 3, value: 64 }
      ],
      min: 0,
      max: 100
    }
  ];
  let selectedRange = $state<[number, number] | null>(null);
</script>

<AnalysisChart
  {tracks}
  valueLabel={(point) => (point?.value === undefined ? 'Sin datos' : `${point.value} °C`)}
  timeLabel={(value) => `Muestra ${value}`}
  legendLabel="Leyenda"
  cursorLabel="Cursor"
  rangeSummaryLabel={(range) => `Rango ${range[0]}–${range[1]}`}
  resetRangeLabel="Borrar rango"
  onRangeSelect={(range) => (selectedRange = range)}
  />
<output data-testid="selected-range">{selectedRange ? selectedRange.join('-') : ''}</output>
