<script lang="ts">
  import { match } from 'ts-pattern';
  import { getThemeContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { Canvas } from './lib/canvas.svelte';
  import Panel from './Panel.svelte';
  import Toolbar from './Toolbar.svelte';
  import Zoom from './Zoom.svelte';

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
    backgroundColor: 'surface.subtle',
  })}
>
  <div style:cursor class={css({ size: 'full', backgroundColor: 'surface.subtle' })}>
    <div bind:this={container} class={css({ size: 'full' })}></div>
  </div>

  {#if canvas}
    <Toolbar {canvas} />
    <Zoom {canvas} />
    <Panel {canvas} />
  {/if}
</div>
