<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { pageRectToClientRect } from '../geometry';
  import { handle } from '../handlers';
  import { handleCopy, handleCut, handlePaste } from '../handlers/clipboard';
  import { handleKeyDown } from '../handlers/keyboard';
  import { IME_CONTEXT_AFTER_LIMIT, IME_CONTEXT_BEFORE_LIMIT, normalizeImeContext } from '../input/ime-context';
  import { ImeInputAdapter } from '../input/ime-input-adapter';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';
  import type { Message } from '@typie/editor-ffi/browser';
  import type { ImeContext, ImeTextInput } from '../input/ime-context';

  const { editor } = getEditorContext();
  const viewportOverlay = getViewportOverlayContext();

  const enqueueMessages = (messages: Message[]) => {
    if (!editor) return;

    let enqueued = false;
    for (const message of messages) {
      editor.enqueue(message);
      enqueued = true;
    }
    if (enqueued) {
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'typewriter' });
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

  const inputRect = $derived.by(() => {
    void viewportOverlay.change;
    const cursor = editor?.cursor;
    if (!editor || !cursor) {
      return null;
    }

    const rect = pageRectToClientRect(editor, { page_idx: cursor.page_idx, rect: cursor.caret });
    if (!rect) return null;

    return {
      left: rect.left,
      top: rect.top,
      width: Math.max(1, rect.width),
      height: Math.max(1, rect.height),
    };
  });

  $effect(() => {
    if (!editor?.focused || !editor.inputEl) return;

    void editor.cursor;
    void editor.selection;
    syncInput();
  });
</script>

{#if editor}
  <textarea
    bind:this={editor.inputEl}
    style:left={`${inputRect?.left ?? -9999}px`}
    style:top={`${inputRect?.top ?? -9999}px`}
    style:width={`${inputRect?.width ?? 1}px`}
    style:height={`${inputRect?.height ?? 1}px`}
    class={css({
      position: 'fixed',
      opacity: '0',
      pointerEvents: 'none',
      resize: 'none',
      overflow: 'hidden',
    })}
    autocapitalize="off"
    autocomplete="off"
    autocorrect="off"
    onbeforeinput={(e) => {
      if (editor.readOnly) return;
      inputAdapter.handleBeforeInput(e as InputEvent & { currentTarget: ImeTextInput });
    }}
    oncompositionend={() => {
      if (editor.readOnly) return;
      inputAdapter.handleCompositionEnd();
    }}
    oncompositionstart={(e) => {
      if (editor.readOnly) return;
      inputAdapter.handleCompositionStart(e as CompositionEvent & { currentTarget: ImeTextInput });
    }}
    oncompositionupdate={(e) => {
      if (editor.readOnly) return;
      inputAdapter.handleCompositionUpdate(e);
    }}
    oncopy={handle(editor, handleCopy)}
    oncut={(e) => {
      if (editor.readOnly) return;
      handleCut(editor, e);
    }}
    onfocus={syncInput}
    oninput={(e) => {
      if (editor.readOnly) return;
      inputAdapter.handleInput(e);
    }}
    onkeydown={(e) => {
      handleKeyDown(editor, e);
    }}
    onpaste={(e) => {
      if (editor.readOnly) return;
      handlePaste(editor, e);
    }}
    spellcheck={false}
  ></textarea>
{/if}
