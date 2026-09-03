<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import View from './View.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Editor_document$key } from '$mearie';
  import type { EditorViewSurfaceLayout } from './editor-view-surface-layout';

  type Props = {
    document$key: Editor_document$key;
    active?: boolean;
    viewer?: boolean;
    useWindowScroll?: boolean;
    style?: SystemStyleObject;
    viewSurfaceLayout?: EditorViewSurfaceLayout;
    header?: Snippet;
    footer?: Snippet;
    placeholderAction?: Snippet;
    children?: Snippet;
    onReady?: () => void;
  };

  let {
    document$key,
    active = true,
    viewer = false,
    useWindowScroll = false,
    style,
    viewSurfaceLayout,
    header,
    footer,
    placeholderAction,
    children,
    onReady,
  }: Props = $props();
</script>

<div
  class={css({
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
    flexGrow: '1',
    ...(!useWindowScroll && {
      overflowY: 'hidden',
    }),
  })}
>
  <View
    style={css.raw({ flex: '1' }, style)}
    {active}
    {document$key}
    {footer}
    {header}
    {onReady}
    {placeholderAction}
    {useWindowScroll}
    {viewSurfaceLayout}
    {viewer}
  >
    {#if children}
      {@render children()}
    {/if}
  </View>
</div>
