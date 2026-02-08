<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';
  import { getActionFromKeyEvent } from '$lib/editor/keyboard';

  type Props = {
    onFocus?: (e: FocusEvent) => void;
    onBlur?: (e: FocusEvent) => void;
  };

  let { onFocus, onBlur }: Props = $props();

  const editor = getEditor();

  let inputEl = $state<HTMLInputElement>();

  let lastInputValue = '';

  let currentCompositionText = '';
  let ignoreEventText = '';

  const resetInputState = () => {
    if (inputEl) {
      inputEl.value = '';
      lastInputValue = '';
    }
  };

  const setClipboardData = (clipboardData: DataTransfer | null, data: { html: string; text: string }) => {
    clipboardData?.setData('text/html', data.html);
    clipboardData?.setData('text/plain', data.text);
  };

  export function focus() {
    inputEl?.focus({ preventScroll: true });
  }

  export function getElement() {
    return inputEl;
  }

  const handleInput = (e: Event) => {
    if (editor.readOnly) return;

    const inputEvent = e as InputEvent;
    if (inputEvent.isComposing) return;

    const value = inputEl?.value || '';

    if (ignoreEventText && value === ignoreEventText) {
      ignoreEventText = '';
      resetInputState();
      return;
    }

    if (!inputEl) return;

    if (!value) {
      lastInputValue = '';
      return;
    }

    if (value.startsWith(lastInputValue) && value.length > lastInputValue.length) {
      // Append
      const newText = value.slice(lastInputValue.length);
      editor.dispatch({ type: 'input', text: newText });
    } else if (lastInputValue.length > 0 && value !== lastInputValue) {
      // Replace (macOS accent popup / text replacement)
      const deleteLength = lastInputValue.length;
      editor.dispatch({ type: 'replaceBackward', length: deleteLength, text: value });
    }

    if (value.length > 64) {
      inputEl.value = value.slice(-64);
    }
    lastInputValue = inputEl.value;
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && editor.contextMenu.isOpen) {
      e.preventDefault();
      editor.closeContextMenu();
      return;
    }

    const action = getActionFromKeyEvent(e);

    const isShortcut = e.ctrlKey || e.metaKey || e.altKey;

    if (e.isComposing) {
      if (action !== null && isShortcut) {
        // 조합 중 단축키 입력 시 preedit을 커밋
        ignoreEventText = currentCompositionText;
        editor.dispatch({ type: 'commitPreedit' });
      } else {
        return;
      }
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

    if (action !== null) {
      e.preventDefault();
      editor.dispatch(action);

      resetInputState();
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
    editor.dispatch({ type: 'deleteBackward' });
  };

  const handlePaste = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    e.preventDefault();

    const html = e.clipboardData?.getData('text/html') || undefined;
    const text = e.clipboardData?.getData('text/plain') ?? '';

    editor.dispatch({ type: 'paste', html, text });
  };

  const handleCompositionStart = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    currentCompositionText = '';
    const text = e.data || '';
    editor.dispatch({ type: 'compositionStart', text });
  };

  const handleCompositionUpdate = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    currentCompositionText = text;

    if (ignoreEventText && text === ignoreEventText) {
      return;
    }

    editor.dispatch({ type: 'compositionUpdate', text });
    editor.typewriter.needsScroll = true;
  };

  const handleCompositionEnd = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    if (ignoreEventText && text === ignoreEventText) {
      ignoreEventText = '';
      editor.dispatch({ type: 'compositionEnd' });
      resetInputState();
      return;
    }

    editor.dispatch({ type: 'input', text });
    editor.dispatch({ type: 'compositionEnd' });

    if (inputEl) {
      const newValue = lastInputValue + text;
      if (newValue.length > 64) {
        inputEl.value = newValue.slice(-64);
      } else {
        inputEl.value = newValue;
      }
      lastInputValue = inputEl.value;
    }
  };

  $effect(() => {
    inputEl?.focus({ preventScroll: true });
  });
</script>

<input
  bind:this={inputEl}
  name="input"
  class={css({ pointerEvents: 'none', position: 'fixed', top: '0', left: '0', height: '1px', width: '1px', opacity: '0' })}
  onblur={onBlur}
  oncompositionend={handleCompositionEnd}
  oncompositionstart={handleCompositionStart}
  oncompositionupdate={handleCompositionUpdate}
  oncopy={handleCopy}
  oncut={handleCut}
  onfocus={onFocus}
  oninput={handleInput}
  onkeydown={handleKeyDown}
  onpaste={handlePaste}
/>
