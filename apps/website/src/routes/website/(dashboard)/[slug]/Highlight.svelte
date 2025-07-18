<script lang="ts">
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let top = $state(0);
  let height = $state(0);

  onMount(() => {
    const container = editor.current.view.dom.closest('.editor') as HTMLElement;
    if (!container) return;

    const handler = () => {
      const { selection } = editor.current.state;
      if (!selection.empty) {
        return;
      }

      const coords = editor.current.view.coordsAtPos(selection.anchor);
      const rect = editor.current.view.dom.getBoundingClientRect();

      top = coords.top - rect.top;
      height = coords.bottom - coords.top;
    };

    document.fonts.ready.then(() => {
      handler();
    });

    editor?.current.on('selectionUpdate', handler);

    return () => {
      editor?.current.off('selectionUpdate', handler);
    };
  });
</script>

<div
  style:top={`${top}px`}
  style:height={`${height}px`}
  class={css({ position: 'absolute', insetX: '-80px', backgroundColor: 'surface.subtle', zIndex: '[-1]' })}
></div>
