<script lang="ts">
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
  };

  let { editor }: Props = $props();

  let visible = $state(false);

  let top = $state(0);
  let height = $state(0);

  onMount(() => {
    const container = editor.current.view.dom.closest('.editor') as HTMLElement;
    if (!container) return;

    const handler = () => {
      const { selection } = editor.current.state;
      if (!selection.empty && !editor.current.view.composing) {
        visible = false;
        return;
      }

      if (!editor.current.isFocused) {
        visible = false;
        return;
      }

      const coords = editor.current.view.coordsAtPos(selection.anchor);
      const rect = editor.current.view.dom.getBoundingClientRect();
      const padding = 4;

      top = coords.top - rect.top - padding;
      height = coords.bottom - coords.top + padding * 2;

      visible = true;
    };

    document.fonts.ready.then(() => {
      handler();
    });

    editor?.current.on('focus', handler);
    editor?.current.on('blur', handler);
    editor?.current.on('selectionUpdate', handler);

    return () => {
      editor?.current.off('focus', handler);
      editor?.current.off('blur', handler);
      editor?.current.off('selectionUpdate', handler);
    };
  });
</script>

{#if visible}
  <div
    style:top={`${top}px`}
    style:height={`${height}px`}
    class={css({ position: 'absolute', insetX: '-80px', backgroundColor: 'surface.subtle', zIndex: '[-1]' })}
  ></div>
{/if}
