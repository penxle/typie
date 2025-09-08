<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { PostLayoutMode } from '@/enums';
  import { GAP_HEIGHT_PX } from '../../tiptap';
  import { mmToPx } from '../../utils/unit';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Ref } from '../../utils';
  import type { PageLayout } from '../../utils/page-layout';

  type Props = {
    layoutMode: Ref<PostLayoutMode>;
    pageLayout?: Ref<PageLayout | undefined>;
    maxWidth: Ref<number>;
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

  // NOTE: 지우면 타임라인에서 제대로 동작하지 않음
  $effect(() => {
    void layoutMode;
    void pageLayout;
    void maxWidth;
  });
</script>

<div
  bind:this={container}
  style:--prosemirror-padding-top={`${bodyPadding.top}px`}
  style:--prosemirror-padding-x={`${bodyPadding.x}px`}
  style:--prosemirror-max-width={layoutMode.current === PostLayoutMode.PAGE && pageLayout?.current
    ? `${mmToPx(pageLayout.current.width)}px`
    : `${maxWidth.current + bodyPadding.x * 2}px`}
  style:--prosemirror-page-margin-top={layoutMode.current === PostLayoutMode.PAGE && pageLayout?.current
    ? `${mmToPx(pageLayout.current.marginTop)}px`
    : '0'}
  style:--prosemirror-page-margin-bottom={layoutMode.current === PostLayoutMode.PAGE && pageLayout?.current
    ? `${mmToPx(pageLayout.current.marginBottom)}px`
    : '0'}
  style:--prosemirror-page-margin-left={layoutMode.current === PostLayoutMode.PAGE && pageLayout?.current
    ? `${mmToPx(pageLayout.current.marginLeft)}px`
    : '0'}
  style:--prosemirror-page-margin-right={layoutMode.current === PostLayoutMode.PAGE && pageLayout?.current
    ? `${mmToPx(pageLayout.current.marginRight)}px`
    : '0'}
  style:--prosemirror-padding-bottom={typewriterEnabled ? '0dvh' : '20dvh'}
  style:--prosemirror-page-gap-height={`${GAP_HEIGHT_PX}px`}
  class={cx(className, css(style))}
  data-layout={layoutMode.current === PostLayoutMode.PAGE ? 'page' : 'scroll'}
>
  {@render children()}
</div>
