<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const { editor } = getEditorContext();

  let point = $state<{ x: number; y: number } | null>(null);

  $effect(() => {
    const cursor = editor?.cursor;
    if (!editor || !cursor) {
      point = null;
      return;
    }

    const { page_idx, caret } = cursor;
    point = editor.localToOffset(page_idx, caret.x, caret.y);
  });

  const transform = $derived.by(() => {
    const scale = editor?.safeDisplayZoom() ?? 1;
    return scale === 1 ? undefined : `scale(${scale})`;
  });
</script>

<div
  style:left={`${point?.x ?? -9999}px`}
  style:top={`${point?.y ?? -9999}px`}
  style:transform
  class={css({ position: 'absolute', transformOrigin: 'top left' })}
>
  {@render children()}
</div>
