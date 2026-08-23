<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import View from './View.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Editor_document$key } from '$mearie';

  type Props = {
    document$key: Editor_document$key;
    active?: boolean;
    viewer?: boolean;
    useWindowScroll?: boolean;
    style?: SystemStyleObject;
    contentInsetLeft?: number;
    contentInsetRight?: number;
    header?: Snippet;
    footer?: Snippet;
    children?: Snippet;
    onReady?: () => void;
  };

  let {
    document$key,
    active = true,
    viewer = false,
    useWindowScroll = false,
    style,
    contentInsetLeft = 0,
    contentInsetRight = 0,
    header,
    footer,
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
    {contentInsetLeft}
    {contentInsetRight}
    {document$key}
    {footer}
    {header}
    {onReady}
    {useWindowScroll}
    {viewer}
  >
    {#if children}
      {@render children()}
    {/if}
  </View>
</div>
