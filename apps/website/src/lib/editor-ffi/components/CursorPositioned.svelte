<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const { editor } = getEditorContext();

  const point = $derived.by(() => {
    if (editor?.cursor) {
      const local = editor.cursor.rect;
      return editor.localToGlobal(editor.cursor.page_idx, local.x, local.y);
    }
  });
</script>

<div style:top={`${point?.y ?? -9999}px`} style:left={`${point?.x ?? -9999}px`} class={css({ position: 'absolute' })}>
  {@render children()}
</div>
