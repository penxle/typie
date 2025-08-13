<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tick, untrack } from 'svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout } from '@typie/ui/tiptap';
  import type { Ref } from '@typie/ui/utils';
  import type { Snippet } from 'svelte';

  type Props = {
    editor: Ref<Editor>;
    children: Snippet;
    pageLayout: PageLayout;
  };

  let { editor, children, pageLayout }: Props = $props();

  const bodyPosition = $derived.by(async () => {
    // NOTE: pageLayout 변경 시에만 재계산
    void pageLayout;

    await tick();

    return untrack(() => {
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
  });
</script>

{#await bodyPosition then position}
  {#if position}
    <div
      style:top={`${position.top}px`}
      style:left={`${position.left}px`}
      style:width={`${position.width}px`}
      class={css({
        position: 'absolute',
        pointerEvents: 'none',
        userSelect: 'none',
      })}
    >
      {@render children()}
    </div>
  {/if}
{/await}
