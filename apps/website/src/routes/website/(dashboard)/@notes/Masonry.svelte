<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
    style?: SystemStyleObject;
    ondragover?: (e: DragEvent) => void;
    ondrop?: (e: DragEvent) => void;
    ondragleave?: (e: DragEvent) => void;
  };

  let { children, style, ondragover, ondrop, ondragleave }: Props = $props();
</script>

<div
  class={css(
    {
      display: 'grid',
      gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))',
      gridAutoRows: '[10px]',
      gridAutoFlow: 'dense',
      gap: '12px',
      width: 'full',
      height: 'fit',
      paddingY: '8px',
      paddingRight: '4px',
    },
    style,
  )}
  {ondragleave}
  ondragover={ondragover || ((e) => e.preventDefault())}
  {ondrop}
  role="list"
>
  {@render children()}
</div>
