<script lang="ts">
  import { css } from '$styled-system/css';
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
    borderColor: 'border.subtle',
    borderRadius: '8px',
    width: 'var(--width)',
    height: 'full',
    backgroundColor: 'surface.default',
    boxShadow: '[0px 4px 12px rgba(0, 0, 0, 0.1)]',
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
