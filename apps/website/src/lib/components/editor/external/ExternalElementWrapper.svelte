<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditor } from '$lib/editor/context';
  import type { Snippet } from 'svelte';
  import type { ExternalElement } from '$lib/editor/types';

  type Props = {
    el: ExternalElement;
    minHeight?: string;
    containerEl?: HTMLDivElement;
    children: Snippet;
  };

  let { el, minHeight = '48px', containerEl = $bindable(), children }: Props = $props();

  const editor = getEditor();

  let reportedHeight = $state<number>();

  $effect(() => {
    if (!containerEl) return;

    const observer = new ResizeObserver((entries) => {
      const height = entries[0].contentRect.height;
      if (height !== reportedHeight && height > 0) {
        reportedHeight = height;
        editor.dispatch({
          type: 'setExternalElementHeight',
          nodeId: el.nodeId,
          height,
        });
      }
    });

    observer.observe(containerEl);
    return () => observer.disconnect();
  });
</script>

<div
  style:left="{el.bounds.x}px"
  style:top="{el.bounds.y}px"
  style:width="{el.bounds.width}px"
  style:min-height={minHeight}
  class={css({
    position: 'absolute',
    userSelect: 'none',
    display: 'flex',
    justifyContent: 'center',
  })}
  data-external-element
  data-node-id={el.nodeId}
>
  <div bind:this={containerEl} class={css({ width: 'full' })}>
    {@render children()}
  </div>

  {#if el.isSelected}
    <div class={css({ position: 'absolute', inset: '0', backgroundColor: 'selection', pointerEvents: 'none' })}></div>
  {/if}
</div>
