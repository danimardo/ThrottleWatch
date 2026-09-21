<script lang="ts">
  import { SegmentedControl } from '../../../components';

  let value = $state('one');
  const options = [
    { value: 'one', label: 'Uno' },
    { value: 'two', label: 'Dos' },
    { value: 'three', label: 'Tres' }
  ];
</script>

<SegmentedControl bind:value label="Opciones" {options} />
<output data-testid="selected">{value}</output>
