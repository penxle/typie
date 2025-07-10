<script lang="ts">
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import { css } from '$styled-system/css';
  import { Canvas } from './class.svelte.ts';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    canvas?: Canvas;
    style?: SystemStyleObject;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
  };

  let { canvas = $bindable(), doc, awareness, style }: Props = $props();

  let element: HTMLDivElement;

  const cursor = $derived.by(() => {
    if (!canvas) return 'default';

    return match(canvas.state.tool)
      .with('pan', () => 'grab')
      .with('select', () => 'default')
      .with('brush', () => 'default')
      .with('arrow', 'line', 'rectangle', 'ellipse', 'stickynote', () => 'crosshair')
      .exhaustive();
  });

  onMount(() => {
    if (!element) return;

    canvas = new Canvas(element, doc, awareness);

    return () => {
      canvas?.destroy();
    };
  });
</script>

<svelte:window onkeydown={(e) => canvas?.handleKeyDown(e)} />

<div style:cursor class={css(style, { backgroundColor: 'gray.50' })}>
  <div
    bind:this={element}
    class={css({ position: 'relative', size: 'full', overflow: 'hidden' })}
    onscroll={() => {
      element.scrollLeft = 0;
      element.scrollTop = 0;
    }}
  ></div>
</div>
