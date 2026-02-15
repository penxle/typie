<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { handleKeyEvent } from '$lib/editor/keyboard';

  type Props = {
    onFocus?: (e: FocusEvent) => void;
    onBlur?: (e: FocusEvent) => void;
  };

  let { onFocus, onBlur }: Props = $props();

  const { editor } = getEditorContext();

  let inputEl = $state<HTMLInputElement>();

  let lastInputValue = '';
  let ignoreEventText = '';

  const resetInputState = () => {
    if (inputEl) {
      inputEl.value = '';
      lastInputValue = '';

      // 강제로 일본어 조합을 끝냄
      inputEl.blur();
      inputEl.focus({ preventScroll: true });

      // 한글 조합 강제로 끝낸 직후 중복으로 들어오는 입력을 방지하되 그 이후의 입력(일본어 변환 포함)은 정상적으로 처리
      setTimeout(() => {
        ignoreEventText = '';
      });
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
      editor.dispatch({ type: 'input', text: newText }).scrollIntoView({ mode: 'typewriter' });
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
      e.preventDefault();
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
    editor.dispatch({ type: 'deleteBackward' }).scrollIntoView({ mode: 'typewriter' });
  };

  const handlePaste = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    e.preventDefault();

    const html = e.clipboardData?.getData('text/html') || undefined;
    const text = e.clipboardData?.getData('text/plain') ?? '';

    if (html && editor.onPaste?.(html, text)) {
      return;
    }

    if (html) {
      editor.dispatch({ type: 'pasteHtml', html, text }).scrollIntoView({ mode: 'typewriter' });
    } else {
      editor.dispatch({ type: 'pasteText', text }).scrollIntoView({ mode: 'typewriter' });
    }
  };

  const handleCompositionStart = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    editor.dispatch({ type: 'compositionStart', text }).scrollIntoView({ mode: 'typewriter' });
  };

  const handleCompositionUpdate = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';

    if (ignoreEventText && text === ignoreEventText) {
      return;
    }

    editor.dispatch({ type: 'compositionUpdate', text }).scrollIntoView({ mode: 'typewriter' });
  };

  const handleCompositionEnd = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    if (ignoreEventText && text === ignoreEventText) {
      ignoreEventText = '';
      editor.dispatch({ type: 'compositionEnd' }).scrollIntoView({ mode: 'typewriter' });
      resetInputState();
      return;
    }

    editor.dispatch({ type: 'commitPreedit' });

    if (inputEl) {
      const newValue = lastInputValue + text;
      if (newValue.length > 64) {
        inputEl.value = newValue.slice(-64);
      } else {
        inputEl.value = newValue;
      }
      lastInputValue = inputEl.value;
    }

    if (pendingKeyEvent) {
      handleKeyEvent(editor, pendingKeyEvent);
      pendingKeyEvent = undefined;
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
  autocapitalize="off"
  autocomplete="off"
  autocorrect="off"
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
  spellcheck={false}
/>
