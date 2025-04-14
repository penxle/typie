<script lang="ts">
  import { portal } from '$lib/actions';
  import { center } from '$styled-system/patterns';
  import Component from './Component.svelte';
  import { store } from './store';
</script>

<div
  class={center({
    position: 'fixed',
    inset: '0',
    zIndex: '[1000]',
    pointerEvents: 'none',
  })}
  use:portal
>
  {#if $store.length > 0}
    <Component dialog={$store[0]} />
  {/if}
</div>
