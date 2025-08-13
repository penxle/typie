<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tick, untrack } from 'svelte';
  import type { Editor } from '@tiptap/core';
  import type { Snippet } from 'svelte';
  import type { PageLayout } from '../tiptap';
  import type { Ref } from '../utils';

  type Props = {
    editor: Ref<Editor>;
    children: Snippet;
    pageLayout: PageLayout;
  };

  let { editor, children, pageLayout }: Props = $props();

  let editorDomSize = $state({ width: 0, height: 0 });
  let bodySize = $state({ width: 0, height: 0 });

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

  $effect(() => {
    const bodyEl = editor.current?.view?.dom?.querySelector('.ProseMirror-body');
    if (!bodyEl) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        bodySize = {
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        };
      }
    });

    resizeObserver.observe(bodyEl);

    return () => {
      resizeObserver.disconnect();
    };
  });

  const bodyPosition = $derived.by(async () => {
    void pageLayout;
    void editorDomSize;
    void bodySize;

    await tick();

    return untrack(() => {
      const bodyElement = editor.current?.view?.dom?.querySelector('.ProseMirror-body') as HTMLElement;

      if (!bodyElement) {
        return null;
      }

      const computedStyle = window.getComputedStyle(bodyElement);
      const paddingLeft = Number.parseFloat(computedStyle.paddingLeft) || 0;
      const paddingRight = Number.parseFloat(computedStyle.paddingRight) || 0;
      const paddingTop = Number.parseFloat(computedStyle.paddingTop) || 0;

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
