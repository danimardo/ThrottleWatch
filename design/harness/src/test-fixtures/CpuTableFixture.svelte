<script lang="ts">
  import { CpuAdvancedTable, type CoreTableRow } from '../../../components';

  const rows: CoreTableRow[] = [
    { id: 'core-0', index: 0, groupLabel: 'P', temperatureLabel: '80 °C', temperatureValue: 80, clockLabel: '4 GHz', clockValue: 4, loadLabel: '20 %', loadValue: 20, throttlingLabel: 'No' },
    { id: 'core-1', index: 1, groupLabel: 'P', temperatureLabel: '60 °C', temperatureValue: 60, clockLabel: '3 GHz', clockValue: 3, loadLabel: '40 %', loadValue: 40, throttlingLabel: 'No' }
  ];
</script>

<CpuAdvancedTable
  {rows}
  columnLabels={{ index: 'Núcleo', group: 'Grupo', temperature: 'Temperatura', clock: 'Reloj', load: 'Carga', throttling: 'Límite' }}
  filterLabel="Filtrar"
  emptyFilterMessage="Sin resultados"
  maxHeight={320}
  />
