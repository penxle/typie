<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import type { RecipeVariantProps } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = {
    style?: SystemStyleObject;
    checked?: boolean;
    children?: Snippet;
  } & Omit<HTMLInputAttributes, 'size' | 'style'> &
    RecipeVariantProps<typeof recipe>;

  let { size = 'lg', style, checked = $bindable(false), children, ...rest }: Props = $props();

  const recipe = cva({
    base: {
      position: 'relative',
      display: 'flex',
      justifyContent: { base: 'flex-start', _checked: 'flex-end' },
      alignItems: 'center',
      borderRadius: 'full',
      aspectRatio: '[2/1]',
      backgroundColor: { base: 'interactive.hover', _checked: 'accent.brand.default' },
      transition: 'common',
      appearance: 'none',
      cursor: 'pointer',
      _after: {
        content: '""',
        borderRadius: 'full',
        height: 'full',
        aspectRatio: '1/1',
        backgroundColor: 'surface.default',
      },
      _disabled: {
        backgroundColor: 'surface.muted!',
        cursor: 'not-allowed',
        _after: { backgroundColor: 'interactive.disabled' },
      },
    },
    variants: {
      size: {
        sm: { width: '36px', height: '18px', padding: '2px', borderRadius: 'full' },
        lg: { width: '40px', height: '20px', padding: '2px', borderRadius: 'full' },
      },
    },
  });
</script>

<label class={css(style)} for={rest['name']}>
  {@render children?.()}
  <input id={rest['name']} class={recipe({ size })} type="checkbox" bind:checked {...rest} />
</label>
