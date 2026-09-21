<script lang="ts">
  import { Dialog, Button } from '../../../components';

  let open = $state(false);
</script>

<button type="button" onclick={() => (open = true)}>Abrir diálogo</button>
<Dialog bind:open title="Confirmar" description="Descripción">
  {#snippet actions()}
    <Button variant="secondary" label="Cancelar" onclick={() => (open = false)} />
    <Button variant="destructive" label="Confirmar" onclick={() => (open = false)} />
  {/snippet}
</Dialog>
