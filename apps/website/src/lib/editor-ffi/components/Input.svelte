<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { handle } from '../handlers';
  import { handleBeforeInput, handleCompositionEnd } from '../handlers/input';
  import { handleKeyDown } from '../handlers/keyboard';

  const { editor } = getEditorContext();
</script>

{#if editor}
  <input
    bind:this={editor.inputEl}
    class={css({ position: 'absolute', left: '0', top: '0', width: '1px', height: '[1em]', opacity: '0', pointerEvents: 'none' })}
    onbeforeinput={handle(editor, handleBeforeInput)}
    oncompositionend={handle(editor, handleCompositionEnd)}
    onkeydown={handle(editor, handleKeyDown)}
  />
{/if}
