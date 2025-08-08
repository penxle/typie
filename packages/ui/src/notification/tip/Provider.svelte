<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { portal } from '../../actions';
  import Item from './Item.svelte';
  import { store } from './store';
</script>

<div
  class={flex({
    direction: 'column',
    gap: '8px',
    position: 'fixed',
    zIndex: 'tip',
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
