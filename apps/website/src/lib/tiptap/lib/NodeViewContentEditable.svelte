<script lang="ts">
  import { css } from '$styled-system/css';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    as?: keyof HTMLElementTagNameMap;
    style?: SystemStyleObject | undefined;
    transparent?: boolean;
    [key: string]: unknown;
  };

  let { as = 'div', style, transparent, ...rest }: Props = $props();
</script>

<svelte:element
  this={as}
  class={css(
    { fontFamily: 'prose', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', wordBreak: 'break-all', userSelect: 'text' },
    style,
  )}
  data-node-view-content-editable
  data-node-view-content-editable-transparent={transparent}
  {...rest}
/>
