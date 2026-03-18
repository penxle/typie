<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_IOS_SAFARI, IS_MAC } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { handleKeyEvent } from '$lib/editor/keyboard';
  import {
    clearBlockedRecomposition,
    clearReconversionCandidate,
    clearReconversionState,
    clearReplacementSourceText,
    countTextChars,
    createReconversionState,
    getCommonSuffix,
    markBlockedRecompositionDelete,
    setReconversionCandidate,
    startBlockedRecomposition,
    suppressBlockedRecomposition,
  } from './ime-reconversion';

  type Props = {
    onFocus?: (e: FocusEvent) => void;
    onBlur?: (e: FocusEvent) => void;
  };

  let { onFocus, onBlur }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  let inputEl = $state<HTMLInputElement>();

  let compositionActive = false;
  let lastInputValue = '';
  let pendingImeDelete = false;
  let lastHandledBackspaceAt = 0;
  let windowFocused = typeof document === 'undefined' ? true : document.hasFocus();
  let documentVisible = typeof document === 'undefined' ? true : document.visibilityState === 'visible';
  let keyEventSerial = 0;
  let lastKeydownWasProcess = false;
  let lastKeydownLooksLikeTypingInput = false;
  let suppressNextBlurCallback = false;
  let pendingCommittedInputSync: string | null = null;
  let compositionStartedFromTypingKey = false;

  let reconversion = $state(createReconversionState());

  const looksLikeTypingInputKeydown = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    if (e.key.length === 1) return true;
    if (e.key !== 'Process' && e.key !== 'Unidentified') return false;
    return /^(Key[A-Z]|Digit[0-9]|Numpad\d)$/.test(e.code);
  };

  const clearInputBuffer = () => {
    if (!inputEl) return;

    inputEl.value = '';
    lastInputValue = '';
    pendingImeDelete = false;
    pendingCommittedInputSync = null;
    clearBlockedRecomposition(reconversion);
  };

  const resetNativeInputSession = () => {
    if (!inputEl) return;

    clearReconversionCandidate(reconversion);
    clearReplacementSourceText(reconversion);
    clearInputBuffer();

    suppressNextBlurCallback = true;
    inputEl.blur();
    inputEl.focus({ preventScroll: true });
  };

  const cancelBlockedRecompositionUi = () => {
    if (!inputEl) return;
    if (typeof document !== 'undefined' && document.activeElement !== inputEl) return;

    suppressNextBlurCallback = true;
    inputEl.blur();
  };

  const resetInputState = () => {
    if (inputEl) {
      clearInputBuffer();

      // 강제로 일본어 조합을 끝냄
      inputEl.blur();
      inputEl.focus({ preventScroll: true });
    }
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

  export function focus() {
    inputEl?.focus({ preventScroll: true });
  }

  export function getElement() {
    return inputEl;
  }

  const handleBeforeInput = (e: InputEvent) => {
    const shouldPreventBlockedRecompositionDelete =
      reconversion.blocked !== null &&
      lastKeydownWasProcess &&
      e.inputType === 'deleteContentBackward' &&
      (inputEl?.value ?? '') === reconversion.blocked.inputValue;
    if (shouldPreventBlockedRecompositionDelete) {
      e.preventDefault();
      pendingImeDelete = false;
      markBlockedRecompositionDelete(reconversion, keyEventSerial, true);
      return;
    }

    if (editor.readOnly || !IS_IOS_SAFARI) return;
    if (e.inputType !== 'deleteContentBackward') return;

    // iOS Safari 한글이 composition* 대신 deleteContentBackward + insertText 를 보냄
    const fromBackspaceKey = Date.now() - lastHandledBackspaceAt < 120;
    if (!fromBackspaceKey) {
      pendingImeDelete = true;
    }
  };

  const handleInput = (e: Event) => {
    const inputEvent = e as InputEvent;

    if (editor.readOnly) return;
    if (inputEvent.isComposing) return;

    const value = inputEl?.value || '';
    if (pendingCommittedInputSync !== null && value === pendingCommittedInputSync) {
      pendingCommittedInputSync = null;
      pendingImeDelete = false;
      lastInputValue = value;
      return;
    }
    pendingCommittedInputSync = null;

    const appendedText = value.startsWith(lastInputValue) ? value.slice(lastInputValue.length) : '';
    const shouldReopenCandidateAfterDelete = reconversion.candidate !== null && inputEvent.inputType === 'deleteContentBackward';
    if (shouldReopenCandidateAfterDelete) {
      pendingImeDelete = false;
      if (reconversion.candidate) {
        reconversion.candidate.reopenDeleteKeyEventSerial = keyEventSerial;
      }
      lastInputValue = value;
      return;
    }

    const shouldSuppressRecompositionAfterDelete =
      reconversion.blocked !== null &&
      inputEvent.inputType === 'deleteContentBackward' &&
      lastInputValue === reconversion.blocked.inputValue;
    if (shouldSuppressRecompositionAfterDelete) {
      pendingImeDelete = false;
      markBlockedRecompositionDelete(reconversion, keyEventSerial);
      lastInputValue = value;
      return;
    }

    const shouldCreateBlockedRecomposition = reconversion.candidate !== null && appendedText.length > 0;
    if (shouldCreateBlockedRecomposition) {
      startBlockedRecomposition(reconversion, value);
    } else if (reconversion.candidate) {
      clearReconversionCandidate(reconversion);
    }

    if (!shouldCreateBlockedRecomposition) {
      clearBlockedRecomposition(reconversion);
    }
    clearReplacementSourceText(reconversion);

    if (!inputEl) return;

    if (!value) {
      if (pendingImeDelete && lastInputValue.length > 0) {
        editor.dispatch({ type: 'replaceBackward', length: lastInputValue.length, text: '' }).scrollIntoView({ mode: 'typewriter' });
      }
      pendingCommittedInputSync = null;
      pendingImeDelete = false;
      lastInputValue = '';
      return;
    }

    pendingImeDelete = false;

    if (value.startsWith(lastInputValue) && value.length > lastInputValue.length) {
      // Append
      const newText = value.slice(lastInputValue.length);
      editor.dispatch({ type: 'input', text: newText }).scrollIntoView({ mode: 'typewriter' });
      if (shouldCreateBlockedRecomposition) {
        resetNativeInputSession();
        return;
      }
    } else if (lastInputValue.length > 0 && value !== lastInputValue) {
      // Replace (macOS accent popup / text replacement)
      const deleteLength = lastInputValue.length;
      editor.dispatch({ type: 'replaceBackward', length: deleteLength, text: value }).scrollIntoView({ mode: 'typewriter' });
    }

    if (value.length > 64) {
      inputEl.value = value.slice(-64);
    }
    lastInputValue = inputEl.value;
  };

  let pendingKeyEvent: KeyboardEvent | undefined;

  const handleKeyDown = (e: KeyboardEvent) => {
    keyEventSerial += 1;
    lastKeydownWasProcess = e.key === 'Process' || e.keyCode === 229;
    lastKeydownLooksLikeTypingInput = looksLikeTypingInputKeydown(e);

    const isModifierOnly = e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt' || e.key === 'Meta';

    if (e.isComposing) {
      if (e.metaKey || e.ctrlKey) {
        pendingKeyEvent = e;
      }

      e.preventDefault();
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

    if (handleKeyEvent(editor, e)) {
      clearReconversionState(reconversion);
      if (IS_IOS_SAFARI && (e.key === 'Backspace' || e.key === 'Delete')) {
        lastHandledBackspaceAt = Date.now();
        pendingImeDelete = false;
      }

      e.preventDefault();
      resetInputState();
      return;
    }

    const shouldPrimeBlockedRecompositionShortcut =
      reconversion.blocked !== null && !compositionActive && !e.isComposing && !isModifierOnly && e.key !== 'Process' && e.key.length > 1;
    if (shouldPrimeBlockedRecompositionShortcut && reconversion.blocked) {
      reconversion.blocked.shortcutPrimed = true;
    }
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
    const suppressBlurCallback = suppressNextBlurCallback;
    suppressNextBlurCallback = false;

    if (reconversion.blocked?.suppressing) {
      compositionActive = false;
      clearReconversionCandidate(reconversion);
      clearReplacementSourceText(reconversion);
      clearInputBuffer();
      requestAnimationFrame(() => {
        if (!inputEl) return;
        if (!windowFocused || !documentVisible) return;
        inputEl.focus({ preventScroll: true });
      });
    } else if (!compositionActive) {
      clearReconversionState(reconversion);
      clearInputBuffer();
    }

    pendingImeDelete = false;
    if (!suppressBlurCallback) {
      onBlur?.(e);
    }
  };

  const handleCompositionStart = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    if (suppressBlockedRecomposition(reconversion, keyEventSerial)) {
      compositionActive = true;
      cancelBlockedRecompositionUi();
      return;
    }

    if (reconversion.blocked) {
      clearBlockedRecomposition(reconversion);
    }

    const text = e.data || '';
    compositionActive = true;
    compositionStartedFromTypingKey = lastKeydownLooksLikeTypingInput;
    editor.dispatch({ type: 'compositionStart', text }).scrollIntoView({ mode: 'typewriter' });
  };

  const handleCompositionUpdate = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    compositionActive = true;

    if (suppressBlockedRecomposition(reconversion, keyEventSerial)) {
      cancelBlockedRecompositionUi();
      return;
    }

    const candidateText = reconversion.candidate?.text ?? '';
    const candidateDeleteSerial = reconversion.candidate?.reopenDeleteKeyEventSerial ?? null;
    const shouldReplaceCommittedCandidate =
      candidateText.length > 0 &&
      (keyEventSerial === candidateDeleteSerial || (!compositionStartedFromTypingKey && text === candidateText));
    if (shouldReplaceCommittedCandidate) {
      pendingKeyEvent = undefined;
      editor.dispatch({ type: 'compositionUpdate', text, replaceLength: countTextChars(candidateText) }).scrollIntoView({
        mode: 'typewriter',
      });
      reconversion.replacementSourceText = candidateText;
      clearReconversionCandidate(reconversion);
      return;
    }

    editor.dispatch({ type: 'compositionUpdate', text }).scrollIntoView({ mode: 'typewriter' });
  };

  const handleCompositionEnd = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const committedText = e.data || inputEl?.value || '';
    const replacementSourceText = reconversion.replacementSourceText;
    if (suppressBlockedRecomposition(reconversion, keyEventSerial)) {
      compositionActive = false;
      compositionStartedFromTypingKey = false;
      pendingKeyEvent = undefined;
      pendingCommittedInputSync = null;
      clearBlockedRecomposition(reconversion);
      clearReplacementSourceText(reconversion);
      clearInputBuffer();
      return;
    }

    editor.dispatch({ type: 'commitPreedit' });
    pendingCommittedInputSync = committedText.length > 0 ? committedText : null;
    if (committedText.length > 0) {
      const nextCandidateText =
        replacementSourceText === null
          ? (reconversion.candidate?.text ?? '') + committedText
          : getCommonSuffix(replacementSourceText, committedText);
      setReconversionCandidate(reconversion, nextCandidateText, keyEventSerial);
    } else if (replacementSourceText !== null) {
      clearReconversionCandidate(reconversion);
    }
    clearReplacementSourceText(reconversion);
    clearBlockedRecomposition(reconversion);

    if (inputEl) {
      inputEl.value = (replacementSourceText === null ? lastInputValue : (reconversion.candidate?.text ?? '')).slice(-64);
      lastInputValue = inputEl.value;
    }
    compositionActive = false;
    compositionStartedFromTypingKey = false;

    if (pendingKeyEvent) {
      handleKeyEvent(editor, pendingKeyEvent);
      pendingKeyEvent = undefined;
    }
  };

  let pointerDownOutsideEditor = false;

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
<svelte:document onvisibilitychange={handleVisibilityChange} />

<input
  bind:this={inputEl}
  name="input"
  class={css({ pointerEvents: 'none', position: 'fixed', top: '0', left: '0', height: '1px', width: '1px', opacity: '0' })}
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
  onfocus={onFocus}
  oninput={handleInput}
  onkeydown={handleKeyDown}
  onpaste={handlePaste}
  spellcheck={false}
/>
