<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { tooltip } from '../actions';
  import type { RecipeVariantProps } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = {
    style?: SystemStyleObject;
    checked?: boolean;
    values?: boolean[];
    children?: Snippet;
  } & Omit<HTMLInputAttributes, 'size' | 'style'> &
    RecipeVariantProps<typeof recipe>;

  let { size = 'lg', style, checked = $bindable(false), values, children, ...rest }: Props = $props();

  const indeterminate = $derived(values?.some((v) => v !== values[0]));

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
      indeterminate: {
        true: {
          justifyContent: 'center',
          backgroundColor: 'accent.brand.subtle',
          _after: {
            width: '1/2',
            aspectRatio: '1/1',
            height: 'auto',
            borderRadius: 'full',
          },
        },
      },
    },
  });
</script>

<label
  class={css(style)}
  for={rest['name']}
  use:tooltip={{ message: indeterminate ? '일부 선택됨' : null, delay: 500, placement: 'right' }}
>
  {@render children?.()}
  <input id={rest['name']} class={recipe({ size, indeterminate })} type="checkbox" bind:checked {...rest} />
</label>
