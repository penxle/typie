<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Toast } from '@typie/ui/notification';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { pageRectToClientRect } from '../geometry';
  import { handle } from '../handlers';
  import { deferPasteShortcutDuringComposition, handleCopy, handleCut, handlePaste, requestPaste } from '../handlers/clipboard';
  import { handleKeyDown } from '../handlers/keyboard';
  import { IME_CONTEXT_AFTER_LIMIT, IME_CONTEXT_BEFORE_LIMIT, normalizeImeContext } from '../input/ime-context';
  import { ImeInputAdapter } from '../input/ime-input-adapter';
  import { wireImeResyncListener } from '../input/ime-resync';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';
  import type { Message } from '@typie/editor-ffi/browser';
  import type { ImeContext, ImeTextInput } from '../input/ime-context';

  const ctx = getEditorContext();
  const { editor } = ctx;
  const viewportOverlay = getViewportOverlayContext();

  const enqueueMessages = (messages: Message[]) => {
    if (!editor || editor.terminal) return;

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
    if (!editor || editor.terminal) return null;

    const ime = editor.ime(IME_CONTEXT_BEFORE_LIMIT, IME_CONTEXT_AFTER_LIMIT);
    return ime ? normalizeImeContext(ime) : null;
  };

  const inputAdapter = new ImeInputAdapter({
    readContext: readEditorImeContext,
    enqueue: enqueueMessages,
  });
  let pendingCompositionDispatch: (() => void) | undefined;
  const handlePasteFailure = ({ file, kind }: { file: File; kind: 'image' | 'file' }) => {
    Toast.error(`${file.name} ${kind === 'image' ? '이미지' : '파일'} 업로드에 실패했습니다.`);
  };

  const syncInput = () => {
    if (!editor?.inputEl || editor.terminal) return;

    inputAdapter.syncFromEditor(editor.inputEl);
  };

  const inputRect = $derived.by(() => {
    void viewportOverlay.change;
    if (!editor || editor.terminal) return null;

    const cursor = editor?.cursor;
    if (!cursor) return null;

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
    if (!editor?.focused || !editor.inputEl || editor.terminal) return;

    void editor.appliedImeRevision;
    syncInput();
  });

  $effect(() => {
    if (!editor || editor.terminal) return;

    return wireImeResyncListener(
      editor,
      inputAdapter,
      () => editor.inputEl ?? null,
      () => {
        pendingCompositionDispatch = undefined;
      },
    );
  });

  $effect(() => {
    if (editor?.readOnly) {
      pendingCompositionDispatch = undefined;
    }
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
      editor.updateNow(() => inputAdapter.handleBeforeInput(e as InputEvent & { currentTarget: ImeTextInput }));
    }}
    onblur={() => {
      pendingCompositionDispatch = undefined;
    }}
    oncompositionend={() => {
      if (editor.readOnly) {
        pendingCompositionDispatch = undefined;
        return;
      }
      const committed = inputAdapter.handleCompositionEnd();
      const action = pendingCompositionDispatch;
      pendingCompositionDispatch = undefined;
      if (committed) {
        action?.();
      }
    }}
    oncompositionstart={(e) => {
      if (editor.readOnly) return;
      pendingCompositionDispatch = undefined;
      inputAdapter.handleCompositionStart(e as CompositionEvent & { currentTarget: ImeTextInput });
    }}
    oncompositionupdate={(e) => {
      if (editor.readOnly) return;
      inputAdapter.handleCompositionUpdate(e);
    }}
    oncopy={handle(editor, handleCopy)}
    oncut={(e) => {
      if (editor.readOnly) {
        editor.editBlockedHandler?.();
        return;
      }
      handleCut(editor, e);
    }}
    onfocus={syncInput}
    oninput={(e) => {
      if (editor.readOnly) return;
      editor.updateNow(() => inputAdapter.handleInput(e));
    }}
    onkeydown={(e) => {
      if (editor.readOnly && e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
        editor.editBlockedHandler?.();
      }
      editor.updateNow(() => {
        pendingCompositionDispatch =
          deferPasteShortcutDuringComposition(
            e,
            () => {
              void requestPaste(ctx, handlePasteFailure);
            },
            () => {
              void editor.requestPasteTextOnly();
            },
          ) ?? handleKeyDown(editor, e);
      });
    }}
    onpaste={(e) => {
      if (editor.readOnly) {
        editor.editBlockedHandler?.();
        return;
      }
      handlePaste(ctx, e, handlePasteFailure);
    }}
    readonly={editor.readOnly}
    spellcheck={false}></textarea>
{/if}
