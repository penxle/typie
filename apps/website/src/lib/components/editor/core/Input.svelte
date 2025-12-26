<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { FRAGMENT_MIME, IS_MAC } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';
  import { getActionFromKeyEvent } from '$lib/editor/keyboard';

  type Props = {
    onFocus?: (e: FocusEvent) => void;
    onBlur?: (e: FocusEvent) => void;
  };

  let { onFocus, onBlur }: Props = $props();

  const editor = getEditor();

  let inputEl = $state<HTMLInputElement>();

  export function focus() {
    inputEl?.focus({ preventScroll: true });
  }

  export function getElement() {
    return inputEl;
  }

  const handleInput = (e: Event) => {
    if (editor.readOnly) return;

    const value = inputEl?.value;
    if (!inputEl || !value || (e as InputEvent).isComposing) return;

    inputEl.value = '';
    editor.dispatch({ type: 'input', text: value });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && editor.contextMenu.isOpen) {
      e.preventDefault();
      editor.closeContextMenu();
      return;
    }

    if (e.isComposing) return;

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

    const action = getActionFromKeyEvent(e);

    if (action !== null) {
      e.preventDefault();
      editor.dispatch(action);
    }
  };

  const handleCopy = (e: ClipboardEvent) => {
    const data = editor.getClipboardData();
    if (!data) return;

    e.preventDefault();
    e.clipboardData?.setData(FRAGMENT_MIME, data.fragment);
    e.clipboardData?.setData('text/html', data.html);
    e.clipboardData?.setData('text/plain', data.text);
  };

  const handleCut = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    const data = editor.getClipboardData();
    if (!data) return;

    e.preventDefault();
    e.clipboardData?.setData(FRAGMENT_MIME, data.fragment);
    e.clipboardData?.setData('text/html', data.html);
    e.clipboardData?.setData('text/plain', data.text);
    editor.dispatch({ type: 'deleteBackward' });
  };

  const handlePaste = (e: ClipboardEvent) => {
    if (editor.readOnly) return;

    e.preventDefault();

    const fragment = e.clipboardData?.getData(FRAGMENT_MIME) || undefined;
    const html = e.clipboardData?.getData('text/html') || undefined;
    const text = e.clipboardData?.getData('text/plain') ?? '';

    editor.dispatch({ type: 'paste', fragment, html, text });
  };

  const handleCompositionStart = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    editor.dispatch({ type: 'compositionStart', text });
  };

  const handleCompositionUpdate = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    const text = e.data || '';
    editor.dispatch({ type: 'compositionUpdate', text });
    editor.typewriter.needsScroll = true;
  };

  const handleCompositionEnd = (e: CompositionEvent) => {
    if (editor.readOnly) return;

    if (inputEl) inputEl.value = '';
    editor.dispatch({ type: 'input', text: e.data || '' });
    editor.dispatch({ type: 'compositionEnd' });
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
