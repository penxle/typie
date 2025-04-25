<script lang="ts">
  import Component from './Component.svelte';
  import { store } from './store';
</script>

{#if $store.length > 0}
  <Component dialog={$store[0]} />
{/if}
