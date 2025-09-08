<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { PostLayoutMode } from '@/enums';
  import { GAP_HEIGHT_PX } from '../../tiptap';
  import { mmToPx } from '../../utils/unit';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
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
    typewriterEnabled?: boolean;
    class?: string;
    style?: SystemStyleObject;
    container?: HTMLDivElement;
    children: Snippet;
  };

  let {
    layoutMode,
    pageLayout,
    maxWidth,
    bodyPadding,
    typewriterEnabled,
    class: className,
    style,
    container = $bindable(),
    children,
  }: Props = $props();
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
  style:--prosemirror-padding-bottom={typewriterEnabled ? '0dvh' : '20dvh'}
  style:--prosemirror-page-gap-height={`${GAP_HEIGHT_PX}px`}
  class={cx(className, css(style))}
  data-layout={layoutMode === PostLayoutMode.PAGE ? 'page' : 'scroll'}
>
  {@render children()}
</div>
