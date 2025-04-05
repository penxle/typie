<script lang="ts">
  import { css, cva } from '$styled-system/css';
  import type { Component } from 'svelte';
  import type { RecipeVariant, SystemStyleObject } from '$styled-system/types';

  type Variants = RecipeVariant<typeof recipe>;

  type Props = {
    icon: Component;
    style?: SystemStyleObject;
    size?: Variants['size'];
    ariaHidden?: boolean;
  };

  let { icon: Icon, style, size = 16, ariaHidden = false }: Props = $props();

  const recipe = cva({
    base: {
      display: 'block',
      verticalAlign: 'middle',
      flex: 'none',
      transition: 'common',
      '& *': { strokeWidth: '[2px]' },
    },
    variants: {
      size: {
        12: { size: '12px' },
        14: { size: '14px' },
        16: { size: '16px' },
        18: { size: '18px' },
        20: { size: '20px' },
        24: { size: '24px' },
        28: { size: '28px' },
        32: { size: '32px' },
      },
    },
  });

  const baseStyle = $derived(recipe.raw({ size }));
</script>

<Icon class={css(baseStyle, style)} aria-hidden={ariaHidden ? 'true' : undefined} />
