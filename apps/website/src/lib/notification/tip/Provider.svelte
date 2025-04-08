<script lang="ts">
  import { portal } from '$lib/actions';
  import { flex } from '$styled-system/patterns';
  import Item from './Item.svelte';
  import { store } from './store';
</script>

<div
  class={flex({
    direction: 'column',
    gap: '8px',
    position: 'fixed',
    zIndex: '[1000]',
    right: '20px',
    bottom: '55px',
    width: '300px',
    pointerEvents: 'none',
  })}
  use:portal
>
  {#each $store as tip (tip.id)}
    <Item {tip} />
  {/each}
</div>
