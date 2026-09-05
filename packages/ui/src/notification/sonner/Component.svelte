<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = {
    // closeToast?: () => void;
    children: Snippet;
  };

  const { children }: Props = $props();
</script>

<div
  class={css({
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '8px',
    width: 'var(--width)',
    height: 'full',
    backgroundColor: 'surface.default',
    boxShadow: 'lg',
    padding: '16px',
    overflow: 'hidden',
  })}
>
  <div
    class={css({
      transition: 'opacity',
      transitionDuration: '400ms',
      '[data-expanded="false"][data-front="false"] &': {
        opacity: '0',
      },
    })}
  >
    {@render children()}
  </div>
</div>
