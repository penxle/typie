<script lang="ts">
  import { css } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

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
