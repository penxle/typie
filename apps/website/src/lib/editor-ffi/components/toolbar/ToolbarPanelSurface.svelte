<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<div
  class={`toolbar-panel-surface ${css({
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'lg',
    overflow: 'hidden',
    _dark: { borderWidth: '1px', borderColor: 'border.hairline' },
  })}`}
>
  {@render children()}
</div>

<style>
  .toolbar-panel-surface {
    transform-origin: left top;
    animation: toolbar-panel-in 150ms cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @keyframes toolbar-panel-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toolbar-panel-surface {
      animation: none;
    }
  }
</style>
