<script lang="ts">
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import MiniDashboardLight from './@dashboard/MiniDashboardLight.svelte';

  let mounted = $state(false);

  onMount(() => {
    mounted = true;
  });
</script>

<div class={css({ backgroundColor: 'gray.50' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'white' })}></div>
</div>

<div class={css({ position: 'relative', width: 'full', height: '[400dvh]' })} data-element="mini-dashboard-container">
  <div
    class={css({
      position: 'sticky',
      top: '0',
      left: '0',
      height: '[100dvh]',
      backgroundColor: 'white',
      overflow: 'hidden',
    })}
    data-element="mini-dashboard-wrapper"
  >
    {#if mounted}
      <div class={css({ position: 'relative', size: 'full', pointerEvents: 'none', fontFamily: 'SUIT' })}>
        <MiniDashboardLight />
      </div>
    {/if}
  </div>
</div>

<div class={css({ backgroundColor: 'dark.gray.700' })}>
  <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'gray.50' })}></div>
</div>
