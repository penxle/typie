<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();

  let element = $state<HTMLDivElement>();
  let point = $state<{ x: number; y: number } | null>(null);

  const cursor = $derived(editor?.cursor);
  const visible = $derived(!!cursor && !!point && !!editor?.focused);

  const resetAnimation = () => {
    element?.getAnimations().forEach((a) => (a.currentTime = 0));
  };

  $effect(() => {
    const el = element;
    if (!editor || !cursor || !el) {
      point = null;
      return;
    }

    const pageEl = editor.pageEls[cursor.page_idx];
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

    const off = editor.on('state_changed', (_, { fields }) => {
      if (fields.includes('cursor')) {
        resetAnimation();
      }
    });

    return () => {
      off();
    };
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
