<script lang="ts">
  import { match } from 'ts-pattern';
  import { getThemeContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import PropertiesPanel from './components/PropertiesPanel.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import Zoom from './components/Zoom.svelte';
  import { Canvas } from './lib/canvas.svelte';

  let container = $state<HTMLDivElement>();
  let canvas = $state<Canvas>();

  const theme = getThemeContext();
  theme.force('light');

  const cursor = $derived.by(() => {
    if (!canvas) return 'default';

    return match(canvas.state.tool)
      .with('pan', () => 'grab')
      .with('select', () => 'default')
      .with('brush', () => 'default')
      .with('arrow', 'line', 'rectangle', 'ellipse', 'stickynote', () => 'crosshair')
      .exhaustive();
  });

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
    backgroundColor: 'gray.50',
  })}
>
  <div style:cursor class={css({ size: 'full', backgroundColor: 'gray.50' })}>
    <div bind:this={container} class={css({ size: 'full' })}></div>
  </div>

  {#if canvas}
    <PropertiesPanel {canvas} />

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
