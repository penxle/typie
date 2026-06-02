<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import View from './View.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Editor_document$key } from '$mearie';

  type Props = {
    document$key: Editor_document$key;
    active?: boolean;
    style?: SystemStyleObject;
    header?: Snippet;
    footer?: Snippet;
    children?: Snippet;
    onReady?: () => void;
  };

  let { document$key, active = true, style, header, footer, children, onReady }: Props = $props();
</script>

<div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowY: 'hidden' })}>
  <View style={css.raw({ flex: '1' }, style)} {active} {document$key} {footer} {header} {onReady}>
    {#if children}
      {@render children()}
    {/if}
  </View>
</div>
