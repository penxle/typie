<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import { Canvas } from './class.svelte.ts';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type * as YAwareness from 'y-protocols/awareness';
  import type * as Y from 'yjs';

  type Props = {
    canvas?: Canvas;
    style?: SystemStyleObject;
    doc?: Y.Doc;
    awareness?: YAwareness.Awareness;
    readonly?: boolean;
  };

  let { canvas = $bindable(), doc, awareness, style, readonly = false }: Props = $props();

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

    // NOTE: readonly 모드일 때는 pan 도구만 사용
    if (readonly && canvas) {
      canvas.state.tool = 'pan';
    }

    return () => {
      canvas?.destroy();
    };
  });
</script>

<svelte:window onkeydown={(e) => canvas?.handleKeyDown(e)} onkeyup={(e) => canvas?.handleKeyUp(e)} />

<div style:cursor class={css(style, { backgroundColor: 'surface.subtle', _dark: { backgroundColor: 'gray.700' } })}>
  <div
    bind:this={element}
    class={css({ position: 'relative', size: 'full', overflow: 'hidden' })}
    onscroll={() => {
      element.scrollLeft = 0;
      element.scrollTop = 0;
    }}
  ></div>
</div>
