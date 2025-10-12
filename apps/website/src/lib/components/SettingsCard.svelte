<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    children: import('svelte').Snippet;
  };

  let { children }: Props = $props();
</script>

<div
  class={css({
    backgroundColor: 'surface.default',
    borderRadius: '8px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    overflow: 'hidden',
  })}
>
  {@render children()}
</div>
