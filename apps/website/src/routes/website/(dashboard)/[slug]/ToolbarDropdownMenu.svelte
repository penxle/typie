<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    children: Snippet;
  };

  let { style, children }: Props = $props();
</script>

<div
  class={css(
    {
      display: 'flex',
      flexDirection: 'column',
      maxHeight: '400px',
      overflowY: 'auto',
      '& > button:not(:first-of-type)': { borderTopWidth: '1px', borderColor: 'border.subtle' },
    },
    style,
  )}
>
  {@render children()}
</div>
