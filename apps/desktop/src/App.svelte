<script lang="ts">
  import Dashboard from './features/dashboard/Dashboard.svelte';
</script>

<h1 class="visually-hidden">ThrottleWatch</h1>
<Dashboard />

<style>
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
