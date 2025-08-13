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

  let editorDomSize = $state({ width: 0, height: 0 });

  $effect(() => {
    const dom = editor.current?.view?.dom;
    if (!dom) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        editorDomSize = {
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        };
      }
    });

    resizeObserver.observe(dom);

    return () => {
      resizeObserver.disconnect();
    };
  });

  const bodyPosition = $derived.by(async () => {
    // NOTE: pageLayout 및 editor DOM 사이즈 변경 시 재계산
    void pageLayout;
    void editorDomSize;

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
