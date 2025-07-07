<script lang="ts">
  import { css } from '$styled-system/css';
  import Toolbar from './components/Toolbar.svelte';
  import Zoom from './components/Zoom.svelte';
  import { Canvas } from './lib/canvas.svelte';

  let container = $state<HTMLDivElement>();
  let canvas = $state<Canvas>();

  $effect(() => {
    if (!container) {
      return;
    }

    canvas = new Canvas(container);

    return () => {
      canvas?.destroy();
    };
  });
</script>

<svelte:window on:keydown={(e) => canvas?.handleKeyDown(e)} />

<div
  class={css({
    position: 'relative',
    width: 'full',
    height: '[100dvh]',
    overflow: 'hidden',
    backgroundColor: 'surface.subtle',
  })}
>
  <div
    bind:this={container}
    class={css({
      width: 'full',
      height: 'full',
      backgroundColor: 'surface.subtle',
    })}
  ></div>

  {#if canvas}
    <div
      class={css({
        position: 'absolute',
        bottom: '24px',
        left: '1/2',
        translate: 'auto',
        translateX: '-1/2',
        zIndex: '10',
      })}
    >
      <Toolbar bind:tool={canvas.state.tool} />
    </div>

    <div
      class={css({
        position: 'absolute',
        top: '20px',
        right: '20px',
        zIndex: '10',
      })}
    >
      <Zoom {canvas} />
    </div>
  {/if}
</div>
