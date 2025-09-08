<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { PostLayoutMode } from '@/enums';
  import { GAP_HEIGHT_PX } from '../../tiptap';
  import { mmToPx } from '../../utils/unit';
  import type { Editor } from '@tiptap/core';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Ref } from '../../utils';
  import type { PageLayout } from '../../utils/page-layout';

  type Props = {
    layoutMode: PostLayoutMode;
    pageLayout?: PageLayout;
    maxWidth: number;
    // NOTE: 맨 앞에 드래그 앤 드랍하는 등의 사용사례를 위해 패딩을 받아서 prosemirror body에 넣어줌으로써 body 영역을 늘려줌
    bodyPadding: {
      top: number;
      x: number;
    };
    typewriterPosition?: number;
    typewriterEnabled?: boolean;
    class?: string;
    style?: SystemStyleObject;
    container?: HTMLDivElement;
    editor?: Ref<Editor>;
    children: Snippet;
  };

  let {
    layoutMode,
    pageLayout,
    maxWidth,
    bodyPadding,
    typewriterPosition = 0.5,
    typewriterEnabled = false,
    class: className,
    style,
    container = $bindable(),
    editor,
    children,
  }: Props = $props();

  const calculateLastPageRemainingHeight = () => {
    if (layoutMode !== PostLayoutMode.PAGE || !pageLayout || !editor?.current) return 0;

    const editorView = editor.current.view;
    if (!editorView) return 0;

    const lastPos = editor.current.state.doc.content.size - 1;
    if (lastPos < 0) return 0;
    const lastItemCoords = editorView.coordsAtPos(lastPos);

    const breakers = editorView.dom.querySelectorAll('[data-page-break="true"] .breaker');
    const lastBreaker = [...breakers].at(-1);
    if (!lastBreaker) {
      return 0;
    }
    const lastBreakerRect = lastBreaker.getBoundingClientRect();

    return lastBreakerRect.bottom - lastItemCoords.bottom;
  };

  const dynamicPaddingBottom = $derived.by(() => {
    if (typewriterEnabled) {
      const typewriterVh = (1 - typewriterPosition) * 100;
      if (layoutMode === PostLayoutMode.PAGE && pageLayout && editor?.current) {
        const remainingHeight = calculateLastPageRemainingHeight();
        return `calc(${typewriterVh}vh - ${remainingHeight}px)`;
      } else {
        return `${typewriterVh}vh`;
      }
    } else {
      return '20dvh';
    }
  });
</script>

<div
  bind:this={container}
  style:--prosemirror-padding-top={`${bodyPadding.top}px`}
  style:--prosemirror-padding-x={`${bodyPadding.x}px`}
  style:--prosemirror-max-width={layoutMode === PostLayoutMode.PAGE && pageLayout
    ? `${mmToPx(pageLayout.width)}px`
    : `${maxWidth + bodyPadding.x * 2}px`}
  style:--prosemirror-page-margin-top={layoutMode === PostLayoutMode.PAGE && pageLayout ? `${mmToPx(pageLayout.marginTop)}px` : '0'}
  style:--prosemirror-page-margin-bottom={layoutMode === PostLayoutMode.PAGE && pageLayout ? `${mmToPx(pageLayout.marginBottom)}px` : '0'}
  style:--prosemirror-page-margin-left={layoutMode === PostLayoutMode.PAGE && pageLayout ? `${mmToPx(pageLayout.marginLeft)}px` : '0'}
  style:--prosemirror-page-margin-right={layoutMode === PostLayoutMode.PAGE && pageLayout ? `${mmToPx(pageLayout.marginRight)}px` : '0'}
  style:--prosemirror-padding-bottom={dynamicPaddingBottom}
  style:--prosemirror-page-gap-height={`${GAP_HEIGHT_PX}px`}
  class={cx(className, css(style))}
  data-layout={layoutMode === PostLayoutMode.PAGE ? 'page' : 'scroll'}
>
  {@render children()}
</div>
