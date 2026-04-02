<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import type { KeyboardEventHandler } from 'svelte/elements';

  const { editor } = getEditorContext();

  const handleKeyDown: KeyboardEventHandler<HTMLInputElement> = (e) => {
    if (!editor) {
      return;
    }

    if (e.key === 'ArrowRight') {
      editor.enqueue({
        type: 'intent',
        value: { type: 'navigation', value: { type: 'move', value: { movement: { type: 'grapheme', value: 'forward' }, extend: false } } },
      });
    } else if (e.key === 'ArrowLeft') {
      editor.enqueue({
        type: 'intent',
        value: { type: 'navigation', value: { type: 'move', value: { movement: { type: 'grapheme', value: 'backward' }, extend: false } } },
      });
    }
  };
</script>

{#if editor}
  <input
    bind:this={editor.inputEl}
    class={css({ position: 'absolute', left: '0', top: '0', width: '1px', height: '[1em]', opacity: '0', pointerEvents: 'none' })}
    onkeydown={handleKeyDown}
  />
{/if}
