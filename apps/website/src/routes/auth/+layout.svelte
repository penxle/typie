<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<div
  style:--grid-line-color={token('colors.decoration.grid.brand')}
  style:--cross-line-color={token('colors.decoration.grid.brand.subtle')}
  style:--grid-size="30px"
  style:--line-thickness="1px"
  class={center({
    padding: '20px',
    width: '[100dvw]',
    minHeight: '[100dvh]',
    height: 'full',
    overflowY: 'auto',
    backgroundColor: 'surface.default',
    backgroundImage:
      '[repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size))]',
    backgroundSize: 'var(--grid-size) var(--grid-size)',
  })}
>
  <div
    class={css({
      borderRadius: '12px',
      padding: { base: '24px', lg: '48px' },
      maxWidth: '400px',
      width: 'full',
      backgroundColor: 'surface.default',
      boxShadow: 'medium',
    })}
  >
    {@render children?.()}
  </div>
</div>
