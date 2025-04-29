<script lang="ts">
  import Component from './Component.svelte';
  import { store } from './store';
</script>

{#if $store.length > 0}
  {#key $store[0].id}
    <Component dialog={$store[0]} />
  {/key}
{/if}
