<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Snippet } from 'svelte';

  type Props = {
    editor: Ref<Editor>;
    children: Snippet;
  };

  let { editor, children }: Props = $props();

  const bodyPosition = $derived.by(() => {
    const bodyElement = editor.current?.view?.dom?.querySelector('.ProseMirror-body') as HTMLElement;

    if (!bodyElement) {
      return null;
    }

    const computedStyle = window.getComputedStyle(bodyElement);
    const paddingLeft = Number.parseInt(computedStyle.paddingLeft, 10) || 0;
    const paddingRight = Number.parseInt(computedStyle.paddingRight, 10) || 0;
    const paddingTop = Number.parseInt(computedStyle.paddingTop, 10) || 0;

    return {
      left: bodyElement.offsetLeft + paddingLeft,
      top: bodyElement.offsetTop + paddingTop,
      width: bodyElement.clientWidth - paddingLeft - paddingRight,
    };
  });
</script>

{#if bodyPosition}
  <div
    style:top={`${bodyPosition.top}px`}
    style:left={`${bodyPosition.left}px`}
    style:width={`${bodyPosition.width}px`}
    class={css({
      position: 'absolute',
      pointerEvents: 'none',
      userSelect: 'none',
    })}
  >
    {@render children()}
  </div>
{/if}
