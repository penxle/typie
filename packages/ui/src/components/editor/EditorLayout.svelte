<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { mmToPx } from '../../utils/unit';
  import type { Snippet } from 'svelte';
  import type { LayoutMode, PageLayout } from '../../utils/page-layout';

  type Props = {
    layoutMode: LayoutMode;
    pageLayout?: PageLayout;
    maxWidth: number;
    typewriterPosition?: number;
    typewriterEnabled?: boolean;
    class?: string;
    mobile?: boolean;
    zoomed?: boolean;
    container?: HTMLDivElement;
    children: Snippet;
  };

  let {
    layoutMode,
    pageLayout,
    maxWidth,
    typewriterPosition = 0.8,
    typewriterEnabled = false,
    class: className,
    mobile = false,
    zoomed = false,
    container = $bindable(),
    children,
  }: Props = $props();
</script>

<div
  bind:this={container}
  style:--prosemirror-max-width={layoutMode === 'page' && pageLayout ? `${mmToPx(pageLayout.width)}px` : `${maxWidth}px`}
  style:--prosemirror-page-margin-top={layoutMode === 'page' && pageLayout ? `${mmToPx(pageLayout.marginTop)}px` : '0'}
  style:--prosemirror-page-margin-bottom={layoutMode === 'page' && pageLayout ? `${mmToPx(pageLayout.marginBottom)}px` : '0'}
  style:--prosemirror-page-margin-left={layoutMode === 'page' && pageLayout ? `${mmToPx(pageLayout.marginLeft)}px` : '0'}
  style:--prosemirror-page-margin-right={layoutMode === 'page' && pageLayout ? `${mmToPx(pageLayout.marginRight)}px` : '0'}
  style:--prosemirror-padding-bottom={mobile
    ? '80dvh'
    : layoutMode === 'page' && pageLayout
      ? '0'
      : typewriterEnabled
        ? `${(1 - typewriterPosition) * 100}vh`
        : '0'}
  class={cx(
    css({
      '&[data-layout="page"]': {
        backgroundColor: 'surface.subtle/50',
        ...(mobile && {
          paddingX: '0',
          touchAction: zoomed ? 'auto' : 'pan-y',
          overflowX: zoomed ? 'auto' : 'hidden',
        }),
      },
    }),
    className,
  )}
  data-layout={layoutMode}
>
  {@render children()}
</div>
