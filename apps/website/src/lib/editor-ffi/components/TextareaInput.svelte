<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Toast } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { caretPageRect, pageRectToClientRect } from '../geometry';
  import { handle } from '../handlers';
  import { deferPasteShortcutDuringComposition, handleCopy, handleCut, handlePaste, requestPaste } from '../handlers/clipboard';
  import { handleKeyDown } from '../handlers/keyboard';
  import { IME_CONTEXT_AFTER_LIMIT, IME_CONTEXT_BEFORE_LIMIT, normalizeImeContext } from '../input/ime-context';
  import { ImeInputAdapter } from '../input/ime-input-adapter';
  import { syncImeInputScroll } from '../input/ime-input-geometry';
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
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
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
  onMount(() =>
    editor?.localEdits.registerInput({
      pending: () => inputAdapter.composing,
      finalize: () => {
        const input = editor.inputEl;
        if (!(input instanceof HTMLTextAreaElement)) return;
        pendingCompositionDispatch = undefined;
        editor.updateNow(() => inputAdapter.finalizeComposition(input));
      },
    }),
  );
  let pendingCompositionDispatch: (() => void) | undefined;
  const handlePasteFailure = ({ file, kind }: { file: File; kind: 'image' | 'file' }) => {
    Toast.error(`${file.name} ${kind === 'image' ? '이미지' : '파일'} 업로드에 실패했습니다.`);
  };

  const syncInput = () => {
    if (!(editor?.inputEl instanceof HTMLTextAreaElement) || editor.terminal) return;

    inputAdapter.syncFromEditor(editor.inputEl);
  };

  const syncInputScroll = () => {
    if (!editor?.focused || !(editor.inputEl instanceof HTMLTextAreaElement) || editor.terminal) return;
    syncImeInputScroll(editor.inputEl);
  };

  const inputRect = $derived.by(() => {
    void viewportOverlay.change;
    if (!editor || editor.terminal) return null;

    const cursor = editor?.cursor;
    if (!cursor) return null;

    const rect = pageRectToClientRect(editor, caretPageRect(cursor));
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
    void inputRect;
    syncInput();
    syncInputScroll();
  });

  $effect(() => {
    if (!editor || editor.terminal) return;

    return wireImeResyncListener(
      editor,
      inputAdapter,
      () => (editor.inputEl instanceof HTMLTextAreaElement ? editor.inputEl : null),
      () => {
        pendingCompositionDispatch = undefined;
      },
    );
  });

  $effect(() => {
    if (editor && !editor.editable) {
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
    style:font-size={`${inputRect?.height ?? 1}px`}
    style:line-height={`${inputRect?.height ?? 1}px`}
    class={css({
      position: 'fixed',
      opacity: '0',
      pointerEvents: 'none',
      resize: 'none',
      overflow: 'hidden',
      whiteSpace: 'pre',
      overflowWrap: 'normal',
    })}
    autocapitalize="off"
    autocomplete="off"
    autocorrect="off"
    onbeforeinput={(e) => {
      if (!editor.editable) {
        e.preventDefault();
        return;
      }
      editor.updateNow(() => inputAdapter.handleBeforeInput(e as InputEvent & { currentTarget: ImeTextInput }));
    }}
    onblur={() => {
      pendingCompositionDispatch = undefined;
    }}
    oncompositionend={() => {
      if (!editor.editable) {
        pendingCompositionDispatch = undefined;
        return;
      }
      const action = pendingCompositionDispatch;
      pendingCompositionDispatch = undefined;
      let committed = false;
      editor.updateNow(() => {
        committed = inputAdapter.handleCompositionEnd();
      });
      editor.localEdits.notify();
      if (committed) action?.();
    }}
    oncompositionstart={(e) => {
      if (!editor.editable) return;
      pendingCompositionDispatch = undefined;
      inputAdapter.handleCompositionStart(e as CompositionEvent & { currentTarget: ImeTextInput });
      editor.localEdits.notify();
    }}
    oncompositionupdate={(e) => {
      if (!editor.editable) return;
      inputAdapter.handleCompositionUpdate(e);
    }}
    oncopy={handle(editor, handleCopy)}
    oncut={(e) => {
      if (!editor.editable) {
        editor.editBlockedHandler?.();
        return;
      }
      handleCut(editor, e);
    }}
    onfocus={syncInput}
    oninput={(e) => {
      if (!editor.editable) return;
      editor.updateNow(() => inputAdapter.handleInput(e));
    }}
    onkeydown={(e) => {
      if (!editor.editable && e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
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
      if (!editor.editable) {
        editor.editBlockedHandler?.();
        return;
      }
      handlePaste(ctx, e, handlePasteFailure);
    }}
    onscroll={syncInputScroll}
    onselect={syncInputScroll}
    readonly={!editor.editable}
    spellcheck={false}></textarea>
{/if}
