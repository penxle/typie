<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { handleKeyEvent } from '$lib/editor/keyboard';
  import {
    createBeforeInputState,
    createCompositionTrackingState,
    createPendingDeltaState,
    createSyntheticTextInputState,
    inferSyntheticTextEditingDeltas,
    invalidateSyntheticTextInputState,
    readDomTextEditingValue,
    reconcileSyntheticTextInputState,
    reduceEditingDeltas,
    resetCompositionTracking,
    startCompositionTracking,
    syncTextEditingValueToElement,
    updateCompositionTracking,
  } from '$lib/editor/synthetic-text-input';
  import type { SyntheticInputMessage } from '$lib/editor/synthetic-text-input';
  import type { Message } from '$lib/editor/types';

  type Props = {
    onFocus?: (e: FocusEvent) => void;
    onBlur?: (e: FocusEvent) => void;
  };

  let { onFocus, onBlur }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  let inputEl = $state<HTMLTextAreaElement>();
  let windowFocused = $state(typeof document === 'undefined' ? true : document.hasFocus());
  let documentVisible = $state(typeof document === 'undefined' ? true : document.visibilityState === 'visible');

  let sessionState = createSyntheticTextInputState();
  let pendingDeltaState = createPendingDeltaState();
  let compositionTracking = createCompositionTrackingState();
  let compositionActive = false;
  let pendingKeyEvent: KeyboardEvent | undefined;
  let pointerDownOutsideEditor = false;
  let imeDebugSequence = 0;

  const MARKER_CODE_START = 0xe0_00;
  const MARKER_CODE_END = 0xf0_00;
  const IME_DEBUG_QUERY_PARAM = 'ime-debug';

  type DebugGlobal = typeof globalThis & {
    __typieImeDebug?: boolean;
    __typieImeLog?: unknown[];
  };

  const imeDebugEnabled = () => {
    if (typeof window === 'undefined') {
      return false;
    }

    const params = new URLSearchParams(window.location.search);
    if (!params.has(IME_DEBUG_QUERY_PARAM)) {
      return false;
    }

    const value = params.get(IME_DEBUG_QUERY_PARAM);
    return value == null || value === '' || value === '1' || value.toLowerCase() === 'true';
  };

  const debugText = (text: string) =>
    [...text]
      .map((char) => {
        const codePoint = char.codePointAt(0) ?? 0;
        if (char === '◆' || char === '\u200B') {
          return '⟪S⟫';
        }

        if (codePoint >= MARKER_CODE_START && codePoint < MARKER_CODE_END) {
          return `⟪M:${codePoint.toString(16)}⟫`;
        }

        return char;
      })
      .join('');

  const serializeValue = (value: {
    text: string;
    selection: { baseOffset: number; extentOffset: number };
    composing: { start: number; end: number };
  }) => ({
    text: debugText(value.text),
    rawText: value.text,
    selection: {
      baseOffset: value.selection.baseOffset,
      extentOffset: value.selection.extentOffset,
    },
    composing: {
      start: value.composing.start,
      end: value.composing.end,
    },
  });

  const serializePendingDelta = () => ({
    inputType: pendingDeltaState.inputType,
    data: pendingDeltaState.data,
    deltaText: debugText(pendingDeltaState.deltaText),
    deltaStart: pendingDeltaState.deltaStart,
    deltaEnd: pendingDeltaState.deltaEnd,
  });

  const serializeCompositionTracking = () => ({
    active: compositionTracking.active,
    text: compositionTracking.text == null ? null : debugText(compositionTracking.text),
    base: compositionTracking.base,
    replaceStart: compositionTracking.replaceStart,
    replaceEnd: compositionTracking.replaceEnd,
  });

  const serializeDomSnapshot = () =>
    inputEl
      ? {
          value: debugText(inputEl.value),
          rawValue: inputEl.value,
          selectionStart: inputEl.selectionStart,
          selectionEnd: inputEl.selectionEnd,
          selectionDirection: inputEl.selectionDirection,
        }
      : null;

  const serializeEditorSelection = () =>
    editor.selection
      ? {
          collapsed: editor.selection.collapsed,
          anchor: editor.selection.anchor,
          head: editor.selection.head,
          precedingText: debugText(editor.selection.precedingText),
          followingText: debugText(editor.selection.followingText),
        }
      : null;

  const pushImeDebugLog = (phase: string, detail: Record<string, unknown> = {}) => {
    const enabled = imeDebugEnabled();
    const debugGlobal = globalThis as DebugGlobal;
    debugGlobal.__typieImeDebug = enabled;
    if (!enabled) {
      return;
    }

    const entry = {
      seq: ++imeDebugSequence,
      phase,
      time: typeof performance === 'undefined' ? Date.now() : performance.now(),
      compositionActive,
      pendingDelta: serializePendingDelta(),
      compositionTracking: serializeCompositionTracking(),
      dom: serializeDomSnapshot(),
      currentValue: serializeValue(sessionState.currentValue),
      editorSelection: serializeEditorSelection(),
      ...detail,
    };

    const buffer = (debugGlobal.__typieImeLog ??= []);
    buffer.push(entry);
    if (buffer.length > 500) {
      buffer.splice(0, buffer.length - 500);
    }

    console.log('[typie-ime]', entry);
  };

  const setClipboardData = (clipboardData: DataTransfer | null, data: { html: string; text: string }) => {
    clipboardData?.setData('text/html', data.html);
    clipboardData?.setData('text/plain', data.text);
  };

  const getClipboardImageFiles = (clipboardData: DataTransfer | null): File[] => {
    if (!clipboardData) {
      return [];
    }

    const imageFiles = [...clipboardData.files].filter((file) => file.type.startsWith('image/'));
    if (imageFiles.length > 0) {
      return imageFiles;
    }

    const files: File[] = [];
    for (const item of clipboardData.items) {
      if (!item.type.startsWith('image/')) continue;
      const file = item.getAsFile();
      if (file) {
        files.push(file);
      }
    }

    return files;
  };

  const dispatchMessages = (messages: SyntheticInputMessage[]) => {
    for (const message of messages) {
      editor.dispatch(message as Message).scrollIntoView({ mode: 'typewriter' });
    }
  };

  const hasComposingRange = (value: {
    composing: {
      start: number;
      end: number;
    };
  }) => value.composing.start >= 0 && value.composing.end > value.composing.start;

  const syncInputToState = () => {
    if (!inputEl) return;
    syncTextEditingValueToElement(inputEl, sessionState.currentValue);
  };

  const processDomInputChange = () => {
    if (!inputEl || editor.readOnly) return;

    pushImeDebugLog('process:start');
    const readResult = readDomTextEditingValue(inputEl, compositionTracking, sessionState.currentValue);
    compositionTracking = readResult.tracking;
    const shouldDeferDomSync = compositionActive || hasComposingRange(readResult.value) || hasComposingRange(sessionState.currentValue);

    const deltas = inferSyntheticTextEditingDeltas(sessionState.currentValue, readResult.value, pendingDeltaState);
    pushImeDebugLog('process:inferred', {
      readValue: serializeValue(readResult.value),
      shouldSyncDom: readResult.shouldSyncDom,
      deltas: deltas.map((delta) => ({
        ...delta,
        oldText: debugText(delta.oldText),
        textInserted: 'textInserted' in delta ? debugText(delta.textInserted) : undefined,
        replacementText: 'replacementText' in delta ? debugText(delta.replacementText) : undefined,
      })),
    });
    pendingDeltaState = createPendingDeltaState();
    if (deltas.length === 0) {
      pushImeDebugLog('process:no-delta', { shouldSyncDom: readResult.shouldSyncDom });
      if (readResult.shouldSyncDom && !shouldDeferDomSync) {
        syncTextEditingValueToElement(inputEl, readResult.value);
        pushImeDebugLog('process:sync-normalized', {
          reason: 'no-delta',
          syncedValue: serializeValue(readResult.value),
        });
      }
      return;
    }

    const reduceResult = reduceEditingDeltas(sessionState, deltas);
    sessionState = reduceResult.state;
    pushImeDebugLog('process:reduced', {
      messages: reduceResult.messages,
      nextValue: serializeValue(reduceResult.state.currentValue),
      domValue: serializeValue(reduceResult.domValue),
    });
    dispatchMessages(reduceResult.messages);

    if (compositionActive || hasComposingRange(readResult.value) || hasComposingRange(reduceResult.state.currentValue)) {
      pushImeDebugLog('process:sync-skipped', { shouldSyncDom: readResult.shouldSyncDom });
      return;
    }

    syncTextEditingValueToElement(inputEl, reduceResult.domValue);
    pushImeDebugLog('process:sync');
  };

  const resetInputState = () => {
    if (!inputEl) return;

    sessionState = invalidateSyntheticTextInputState(sessionState);
    pendingDeltaState = createPendingDeltaState();
    compositionTracking = resetCompositionTracking();
    compositionActive = false;
    const reconcileResult = reconcileSyntheticTextInputState(sessionState, editor.selection);
    sessionState = reconcileResult.state;
    syncTextEditingValueToElement(inputEl, reconcileResult.domValue);

    if (reconcileResult.shouldCommitPreedit) {
      editor.dispatch({ type: 'commitPreedit' }).scrollIntoView({ mode: 'typewriter' });
    }
  };

  const replayDeferredKeyEvent = () => {
    if (!pendingKeyEvent) {
      return;
    }

    const deferredEvent = pendingKeyEvent;
    pendingKeyEvent = undefined;
    if (handleKeyEvent(editor, deferredEvent)) {
      resetInputState();
    }
  };

  export function focus() {
    inputEl?.focus({ preventScroll: true });
  }

  export function getElement() {
    return inputEl;
  }

  const handleBeforeInput = (e: InputEvent) => {
    if (editor.readOnly) return;

    pendingDeltaState = createBeforeInputState(sessionState.currentValue, e.inputType ?? null, e.data ?? null);
    pushImeDebugLog('event:beforeinput', {
      inputType: e.inputType ?? null,
      data: e.data ?? null,
      isComposing: e.isComposing,
    });
  };

  const handleInput = () => {
    if (editor.readOnly) return;
    pushImeDebugLog('event:input');
    processDomInputChange();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    pushImeDebugLog('event:keydown', {
      key: e.key,
      code: e.code,
      isComposing: e.isComposing,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
      altKey: e.altKey,
      shiftKey: e.shiftKey,
    });

    if (e.isComposing) {
      const isModifierOnly = e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt' || e.key === 'Meta';
      if ((e.metaKey || e.ctrlKey) && !isModifierOnly) {
        pendingKeyEvent = e;
        e.preventDefault();
      }
      return;
    }

    if (editor.contextMenu.isOpen) {
      e.preventDefault();
      return;
    }

    if (IS_MAC && e.ctrlKey) {
      const key = e.key.toLowerCase();

      if (key === 'q' || key === 'ㅂ') {
        console.log(editor.inspectState());
        return;
      }

      if (key === 'w' || key === 'ㅈ') {
        console.log(editor.inspectStateAsMacro());
        return;
      }

      if (key === 'f' || key === 'ㄹ') {
        console.log(editor.inspectSelectionAsFragmentMacro());
        return;
      }
    }

    if (!handleKeyEvent(editor, e)) {
      return;
    }

    e.preventDefault();
    resetInputState();
  };

  const handleCopy = (e: ClipboardEvent) => {
    const data = editor.getClipboardData();
    if (!data) return;

    e.preventDefault();
    setClipboardData(e.clipboardData, data);
  };

  const handleCut = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    const data = editor.getClipboardData();
    if (!data) return;

    e.preventDefault();
    setClipboardData(e.clipboardData, data);
    editor.dispatch({ type: 'deleteBackward' }).scrollIntoView({ mode: 'typewriter' });
  };

  const handlePaste = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    e.preventDefault();

    const imageFiles = getClipboardImageFiles(e.clipboardData);
    if (editor.insertImagesFromFiles(imageFiles)) {
      return;
    }

    const html = e.clipboardData?.getData('text/html') || undefined;
    const text = e.clipboardData?.getData('text/plain') ?? '';

    editor.paste({ html, text });
  };

  const handleBlur = (e: FocusEvent) => {
    if (compositionActive) {
      pushImeDebugLog('event:blur:commit-preedit');
      processDomInputChange();
      editor.dispatch({ type: 'commitPreedit' }).scrollIntoView({ mode: 'typewriter' });
    }

    sessionState = invalidateSyntheticTextInputState(sessionState);
    pendingDeltaState = createPendingDeltaState();
    compositionTracking = resetCompositionTracking();
    compositionActive = false;
    pushImeDebugLog('event:blur', {
      relatedTarget: e.relatedTarget instanceof HTMLElement ? e.relatedTarget.tagName : null,
    });

    onBlur?.(e);
  };

  const handleFocus = (e: FocusEvent) => {
    syncInputToState();
    pushImeDebugLog('event:focus');
    onFocus?.(e);
  };

  const handleCompositionStart = () => {
    if (editor.readOnly) return;

    compositionActive = true;
    const selectionStart = inputEl?.selectionStart;
    const selectionEnd = inputEl?.selectionEnd;
    compositionTracking =
      selectionStart == null || selectionEnd == null
        ? startCompositionTracking()
        : startCompositionTracking({
            start: Math.min(selectionStart, selectionEnd),
            end: Math.max(selectionStart, selectionEnd),
          });
    pushImeDebugLog('event:compositionstart');
  };

  const handleCompositionUpdate = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    compositionActive = true;
    compositionTracking = updateCompositionTracking(compositionTracking, e.data);
    pushImeDebugLog('event:compositionupdate', {
      data: e.data == null ? null : debugText(e.data),
    });
  };

  const handleCompositionEnd = () => {
    if (editor.readOnly) return;

    compositionActive = false;
    compositionTracking = resetCompositionTracking();
    syncInputToState();
    pushImeDebugLog('event:compositionend');

    queueMicrotask(() => {
      pushImeDebugLog('event:compositionend:microtask');
      processDomInputChange();
      replayDeferredKeyEvent();
    });
  };

  const handleSelectionChange = () => {
    if (editor.readOnly || !inputEl) return;
    if (document.activeElement !== inputEl) return;

    // IME-driven selection updates inside the preedit range should not dispatch editor messages.
    if (compositionActive || hasActiveCompositionInState()) {
      pushImeDebugLog('event:selectionchange:skipped');
      return;
    }

    pushImeDebugLog('event:selectionchange');
    processDomInputChange();
  };

  const hasActiveCompositionInState = () =>
    sessionState.currentValue.composing.start >= 0 && sessionState.currentValue.composing.end > sessionState.currentValue.composing.start;

  $effect(() => {
    if (!inputEl) return;

    // During IME composition, the browser owns the live DOM value.
    // Reconcile is only safe once composition has settled.
    if (compositionActive || hasActiveCompositionInState()) {
      pushImeDebugLog('reconcile:skip');
      return;
    }

    const selection = editor.selection;
    const reconcileResult = reconcileSyntheticTextInputState(sessionState, selection);
    sessionState = reconcileResult.state;
    pushImeDebugLog('reconcile:apply', {
      shouldCommitPreedit: reconcileResult.shouldCommitPreedit,
      reconciledValue: serializeValue(reconcileResult.state.currentValue),
    });
    syncTextEditingValueToElement(inputEl, reconcileResult.domValue);

    if (reconcileResult.shouldCommitPreedit) {
      editor.dispatch({ type: 'commitPreedit' }).scrollIntoView({ mode: 'typewriter' });
    }
  });

  $effect(() => {
    const paneEl = editor.scrollContainerEl?.closest('[data-pane-id]') as HTMLElement | null;
    if (!paneEl) return;

    const handlePointerDown = (e: PointerEvent) => {
      if (editor.extensionArea.containerEl?.contains(e.target as Node)) return;
      pointerDownOutsideEditor = true;
      requestAnimationFrame(() => {
        pointerDownOutsideEditor = false;
      });
    };

    paneEl.addEventListener('pointerdown', handlePointerDown, true);
    return () => paneEl.removeEventListener('pointerdown', handlePointerDown, true);
  });

  $effect(() => {
    if (!ctx.paneFocused) return;
    if (!editor.isFocused) return;
    if (pointerDownOutsideEditor) return;
    if (!windowFocused || !documentVisible) return;
    if (typeof document !== 'undefined' && document.activeElement === inputEl) return;

    inputEl?.focus({ preventScroll: true });
    syncInputToState();
  });

  const handleWindowBlur = () => {
    windowFocused = false;
  };

  const handleWindowFocus = () => {
    windowFocused = true;
  };

  const handleVisibilityChange = () => {
    documentVisible = document.visibilityState === 'visible';
    windowFocused = document.hasFocus();
  };
</script>

<svelte:window onblur={handleWindowBlur} onfocus={handleWindowFocus} />
<svelte:document onselectionchange={handleSelectionChange} onvisibilitychange={handleVisibilityChange} />

<textarea
  bind:this={inputEl}
  name="input"
  class={css({
    pointerEvents: 'none',
    position: 'fixed',
    top: '0',
    left: '0',
    height: '1px',
    width: '1px',
    opacity: '0',
    resize: 'none',
    overflow: 'hidden',
  })}
  autocapitalize="off"
  autocomplete="off"
  autocorrect="off"
  onbeforeinput={handleBeforeInput}
  onblur={handleBlur}
  oncompositionend={handleCompositionEnd}
  oncompositionstart={handleCompositionStart}
  oncompositionupdate={handleCompositionUpdate}
  oncopy={handleCopy}
  oncut={handleCut}
  onfocus={handleFocus}
  oninput={handleInput}
  onkeydown={handleKeyDown}
  onpaste={handlePaste}
  spellcheck={false}
></textarea>
