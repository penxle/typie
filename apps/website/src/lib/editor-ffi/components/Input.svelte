<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { handle } from '../handlers';
  import { handleCopy, handleCut, handlePaste } from '../handlers/clipboard';
  import { handleKeyDown } from '../handlers/keyboard';
  import { IME_CONTEXT_AFTER_LIMIT, IME_CONTEXT_BEFORE_LIMIT, normalizeImeContext } from '../input/ime-context';
  import { ImeInputAdapter } from '../input/ime-input-adapter';
  import type { Message } from '@typie/editor-ffi/browser';
  import type { ImeContext } from '../input/ime-context';

  const { editor } = getEditorContext();

  const enqueueMessages = (messages: Message[]) => {
    if (!editor) return;

    for (const message of messages) {
      editor.enqueue(message);
    }
  };

  const readEditorImeContext = (): ImeContext | null => {
    if (!editor) return null;

    try {
      return normalizeImeContext(editor.ime(IME_CONTEXT_BEFORE_LIMIT, IME_CONTEXT_AFTER_LIMIT));
    } catch {
      return null;
    }
  };

  const inputAdapter = new ImeInputAdapter({
    readContext: readEditorImeContext,
    enqueue: enqueueMessages,
  });

  const syncInput = () => {
    if (!editor?.inputEl) return;

    inputAdapter.syncFromEditor(editor.inputEl);
  };

  $effect(() => {
    if (!editor?.focused || !editor.inputEl) return;

    void editor.cursor;
    void editor.selection;
    syncInput();
  });
</script>

{#if editor}
  <input
    bind:this={editor.inputEl}
    class={css({ position: 'absolute', left: '0', top: '0', width: '1px', height: '[1em]', opacity: '0', pointerEvents: 'none' })}
    onbeforeinput={(e) => inputAdapter.handleBeforeInput(e as InputEvent & { currentTarget: HTMLInputElement })}
    oncompositionend={() => inputAdapter.handleCompositionEnd()}
    oncompositionstart={(e) => inputAdapter.handleCompositionStart(e as CompositionEvent & { currentTarget: HTMLInputElement })}
    oncompositionupdate={(e) => inputAdapter.handleCompositionUpdate(e)}
    oncopy={handle(editor, handleCopy)}
    oncut={handle(editor, handleCut)}
    onfocus={syncInput}
    oninput={(e) => inputAdapter.handleInput(e)}
    onkeydown={handle(editor, handleKeyDown)}
    onpaste={handle(editor, handlePaste)}
  />
{/if}
