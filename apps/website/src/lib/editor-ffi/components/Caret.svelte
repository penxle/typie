<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import { presentedPageElement } from '../geometry';
  import { isCaretVisible } from './caret-visibility';

  const { editor } = getEditorContext();

  let element = $state<HTMLDivElement>();
  let point = $state<{ x: number; y: number } | null>(null);

  const cursor = $derived.by(() => {
    const current = editor?.cursor;
    return current && editor && presentedPageElement(editor, current.page_idx) ? current : undefined;
  });
  const visible = $derived(
    !!editor && isCaretVisible({ hasCursor: !!cursor, hasPoint: !!point, focused: editor.focused, readOnly: editor.readOnly }),
  );

  const resetAnimation = () => {
    for (const a of element?.getAnimations() ?? []) {
      a.currentTime = 0;
    }
  };

  $effect(() => {
    const el = element;
    if (!editor || !cursor || !el) {
      point = null;
      return;
    }

    const pageEl = presentedPageElement(editor, cursor.page_idx);
    if (!pageEl) {
      point = null;
      return;
    }

    if (el.parentElement !== pageEl) {
      pageEl.append(el);
    }

    point = { x: cursor.caret.x, y: cursor.caret.y };
  });

  $effect(() => {
    if (!editor || !element) return;
    void cursor;
    resetAnimation();
  });
</script>

<div
  bind:this={element}
  style:left={`${point?.x ?? -9999}px`}
  style:top={`${point?.y ?? -9999}px`}
  style:width={`${cursor?.caret.width ?? 1}px`}
  style:height={`${cursor?.caret.height ?? 0}px`}
  style:visibility={visible ? 'visible' : 'hidden'}
  class={css({
    position: 'absolute',
    backgroundColor: 'text.default',
    animation: 'blink 1s step-end infinite',
    pointerEvents: 'none',
  })}
  data-editor-caret
></div>

<style>
  @keyframes -global-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
</style>
