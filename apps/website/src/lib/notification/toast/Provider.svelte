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
    bottom: '20px',
    paddingLeft: '20px',
    pointerEvents: 'none',
  })}
  use:portal
>
  {#each $store as toast (toast.id)}
    <Item {toast} />
  {/each}
</div>
