<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const { editor } = getEditorContext();

  const pageContainer = $derived(editor?.cursor ? editor.pageEls[editor.cursor.page_idx] : undefined);
  let element = $state<HTMLDivElement>();

  const point = $derived.by(() => {
    if (editor?.cursor) {
      const local = editor.cursor.caret;
      return { x: local.x, y: local.y };
    }
  });

  $effect(() => {
    if (pageContainer && element && element.parentElement !== pageContainer) {
      pageContainer.append(element);
    }
  });
</script>

<div bind:this={element} style:top={`${point?.y ?? -9999}px`} style:left={`${point?.x ?? -9999}px`} class={css({ position: 'absolute' })}>
  {@render children()}
</div>
