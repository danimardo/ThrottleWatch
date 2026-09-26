<script lang="ts">
  import AppShell from './shell/AppShell.svelte';
  import { localeStore } from './lib/i18n/runtime';
</script>

<!-- A language change rebuilds the whole interface: each screen builds its texts when it is created,
     so a new language means new screens. AppShell keeps the current destination across that. -->
{#key $localeStore}
  <AppShell />
{/key}
