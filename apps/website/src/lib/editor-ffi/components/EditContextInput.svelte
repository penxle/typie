<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Toast } from '@typie/ui/notification';
  import { untrack } from 'svelte';
  import { getEditorContext } from '../editor.svelte';
  import { pageRectToClientRect } from '../geometry';
  import { handle } from '../handlers';
  import { deferPasteShortcutDuringComposition, handleCopy, handleCut, handlePaste, requestPaste } from '../handlers/clipboard';
  import { handleKeyDown } from '../handlers/keyboard';
  import { EditContextInputAdapter } from '../input/edit-context-input-adapter';
  import { IME_CONTEXT_AFTER_LIMIT, IME_CONTEXT_BEFORE_LIMIT, normalizeImeContext } from '../input/ime-context';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';

  const ctx = getEditorContext();
  const { editor } = ctx;
  const viewportOverlay = getViewportOverlayContext();
  let element = $state<HTMLDivElement>();
  let pendingCompositionDispatch: (() => void) | undefined;
  let boundsRequest: { start: number; end: number } | undefined;
  let resyncing = false;

  const adapter = new EditContextInputAdapter(
    () => {
      if (!editor || editor.terminal) return null;
      const ime = editor.ime(IME_CONTEXT_BEFORE_LIMIT, IME_CONTEXT_AFTER_LIMIT);
      return ime ? normalizeImeContext(ime) : null;
    },
    (messages) => {
      if (!editor || editor.terminal || editor.readOnly) return;
      for (const message of messages) editor.enqueue(message);
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
    },
  );
  const editContext = adapter.editContext;

  const clientRect = (start: number, end: number): DOMRect | null => {
    if (!editor || editor.terminal || editor.published?.snapshot.revision !== editor.appliedRevision) return null;
    const rect = editor.firstRectForRange(start, end);
    return rect ? pageRectToClientRect(editor, rect) : null;
  };

  const inputRect = $derived.by(() => {
    void viewportOverlay.change;
    if (!editor || editor.terminal || !editor.cursor) return null;
    return pageRectToClientRect(editor, { page_idx: editor.cursor.page_idx, rect: editor.cursor.caret });
  });

  const syncGeometry = () => {
    if (!editor?.focused || editor.terminal || editor.published?.snapshot.revision !== editor.appliedRevision) {
      return;
    }
    const selectionStart = adapter.flatOffset(Math.min(editContext.selectionStart, editContext.selectionEnd));
    const selectionEnd = adapter.flatOffset(Math.max(editContext.selectionStart, editContext.selectionEnd));
    const selection = clientRect(selectionStart, selectionEnd);
    if (selection) editContext.updateSelectionBounds(selection);
    const control = editor.scrollContainerEl?.getBoundingClientRect();
    if (control) editContext.updateControlBounds(control);

    if (boundsRequest) {
      const bounds: DOMRect[] = [];
      for (let index = boundsRequest.start; index < boundsRequest.end; index += 1) {
        const start = adapter.flatOffset(index);
        const rect = clientRect(start, start + 1);
        if (!rect) return;
        // EditContext indexes UTF-16 code units, while the engine indexes Unicode
        // scalars. Both halves of a surrogate pair use the same character bounds.
        bounds.push(rect);
      }
      editContext.updateCharacterBounds(boundsRequest.start, bounds);
    }
  };

  const syncInput = () => {
    if (!editor || editor.terminal) return;
    adapter.syncFromEditor();
    syncGeometry();
  };

  $effect(() => {
    if (!editor || !element || editor.terminal) return;
    const input = element;
    editor.inputEl = input;
    const listeners = new AbortController();
    const options = { signal: listeners.signal };
    editContext.addEventListener(
      'textupdate',
      (event) => {
        if (resyncing || editor.readOnly || editor.terminal) return;
        editor.updateNow(() => adapter.handleTextUpdate(event));
        syncInput();
      },
      options,
    );
    editContext.addEventListener(
      'compositionstart',
      () => {
        if (resyncing || editor.readOnly) return;
        pendingCompositionDispatch = undefined;
        adapter.handleCompositionStart();
      },
      options,
    );
    editContext.addEventListener(
      'compositionend',
      () => {
        if (resyncing) return;
        boundsRequest = undefined;
        const action = pendingCompositionDispatch;
        pendingCompositionDispatch = undefined;
        let committed = false;
        if (!editor.terminal)
          editor.updateNow(() => {
            committed = adapter.handleCompositionEnd();
          });
        if (committed) action?.();
        // Chromium clears its native composition after dispatching this event.
        // Updating its selection inside the callback can cancel that composition.
        queueMicrotask(() => {
          if (editor.inputEl === input) syncInput();
        });
      },
      options,
    );
    editContext.addEventListener(
      'textformatupdate',
      (event) => {
        if (resyncing || editor.readOnly || editor.terminal) return;
        const ranges = adapter.composing
          ? event
              .getTextFormats()
              .filter((format) => format.underlineStyle !== 'none' && format.underlineThickness === 'thick')
              .map((format) => ({ start: adapter.flatOffset(format.rangeStart), end: adapter.flatOffset(format.rangeEnd) }))
          : [];
        editor.updateNow(() => editor.enqueue({ type: 'view', op: { type: 'set_composition_target_ranges', ranges } }));
        syncGeometry();
      },
      options,
    );
    editContext.addEventListener(
      'characterboundsupdate',
      (event) => {
        boundsRequest = { start: event.rangeStart, end: event.rangeEnd };
        syncGeometry();
      },
      options,
    );
    const dispose = editor.on('ime_resync_required', () => {
      pendingCompositionDispatch = undefined;
      queueMicrotask(() => {
        if (editor.terminal || editor.inputEl !== input) return;
        const focused = document.activeElement === input;
        resyncing = true;
        try {
          input.editContext = null;
          adapter.resetForResync();
          boundsRequest = undefined;
          if (!editor.readOnly) input.editContext = editContext;
          if (focused) input.focus({ preventScroll: true });
        } finally {
          resyncing = false;
        }
      });
    });
    return () => {
      dispose();
      listeners.abort();
      input.editContext = null;
      if (editor.inputEl === input) editor.inputEl = undefined;
    };
  });

  $effect(() => {
    if (!element || !editor || editor.terminal) return;
    element.editContext = editor.readOnly ? null : editContext;
    if (editor.readOnly) {
      pendingCompositionDispatch = undefined;
      untrack(() => editor.updateNow(() => editor.enqueue({ type: 'text_input', ops: [{ type: 'clear_composition' }] })));
    }
  });

  $effect(() => {
    if (!editor?.focused || editor.terminal) return;
    void editor.appliedImeRevision;
    void viewportOverlay.change;
    void editor.published;
    untrack(syncInput);
  });

  const handlePasteFailure = ({ file, kind }: { file: File; kind: 'image' | 'file' }) => {
    Toast.error(`${file.name} ${kind === 'image' ? '이미지' : '파일'} 업로드에 실패했습니다.`);
  };
</script>

{#if editor}
  <div
    bind:this={element}
    style:left={`${inputRect?.left ?? -9999}px`}
    style:top={`${inputRect?.top ?? -9999}px`}
    style:width={`${Math.max(1, inputRect?.width ?? 1)}px`}
    style:height={`${Math.max(1, inputRect?.height ?? 1)}px`}
    class={css({ position: 'fixed', opacity: '0', pointerEvents: 'none' })}
    aria-multiline="true"
    aria-readonly={editor.readOnly}
    onbeforeinput={(event) => {
      if (editor.readOnly) {
        event.preventDefault();
        return;
      }
      editor.updateNow(() => adapter.handleBeforeInput(event));
    }}
    onblur={() => {
      pendingCompositionDispatch = undefined;
    }}
    oncopy={handle(editor, handleCopy)}
    oncut={(event) => {
      if (editor.readOnly) {
        editor.editBlockedHandler?.();
        return;
      }
      handleCut(editor, event);
    }}
    onfocus={syncInput}
    onkeydown={(event) => {
      if (editor.readOnly && event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
        editor.editBlockedHandler?.();
      }
      // EditContext owns composition independently of DOM composition events;
      // Chromium can send keydown with isComposing=false during native preedit.
      const composing = adapter.composing || event.isComposing;
      editor.updateNow(() => {
        pendingCompositionDispatch =
          deferPasteShortcutDuringComposition(
            event,
            () => {
              void requestPaste(ctx, handlePasteFailure);
            },
            () => {
              void editor.requestPasteTextOnly();
            },
            composing,
          ) ?? handleKeyDown(editor, event, composing);
      });
      if (element && pendingCompositionDispatch && event.defaultPrevented) {
        // Unlike a textarea, EditContext has no native default action after a
        // consumed shortcut. Detaching it commits preedit before the action runs.
        element.editContext = null;
        element.editContext = editContext;
      }
    }}
    onpaste={(event) => {
      if (editor.readOnly) {
        editor.editBlockedHandler?.();
        return;
      }
      handlePaste(ctx, event, handlePasteFailure);
    }}
    role="textbox"
    tabindex="0"
  ></div>
{/if}
