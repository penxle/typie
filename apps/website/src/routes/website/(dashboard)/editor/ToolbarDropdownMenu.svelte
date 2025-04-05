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
      backgroundColor: 'white',
      borderWidth: '1px',
      borderRadius: '4px',
      '& > button:not(:first-of-type)': { borderTopWidth: '1px', borderTopColor: 'gray.100' },
    },
    style,
  )}
>
  {@render children()}
</div>
