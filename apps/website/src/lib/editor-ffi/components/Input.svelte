<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { handle } from '../handlers';
  import { handleCopy, handleCut, handlePaste } from '../handlers/clipboard';
  import { handleBeforeInput, handleCompositionEnd, handleCompositionStart } from '../handlers/input';
  import { handleKeyDown } from '../handlers/keyboard';

  const { editor } = getEditorContext();
</script>

{#if editor}
  <input
    bind:this={editor.inputEl}
    class={css({ position: 'absolute', left: '0', top: '0', width: '1px', height: '[1em]', opacity: '0', pointerEvents: 'none' })}
    onbeforeinput={handle(editor, handleBeforeInput)}
    oncompositionend={handle(editor, handleCompositionEnd)}
    oncompositionstart={handle(editor, handleCompositionStart)}
    oncopy={handle(editor, handleCopy)}
    oncut={handle(editor, handleCut)}
    onkeydown={handle(editor, handleKeyDown)}
    onpaste={handle(editor, handlePaste)}
  />
{/if}
