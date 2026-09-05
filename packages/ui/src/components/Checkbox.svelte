<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import type { RecipeVariantProps } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = Omit<HTMLInputAttributes, 'size' | 'style'> &
    RecipeVariantProps<typeof recipe> & {
      style?: SystemStyleObject;
      checked?: boolean;
      children?: Snippet;
      label?: string;
      clickPadding?: boolean;
    };

  let {
    size = 'md',
    variant = 'brand',
    style,
    checked = $bindable(false),
    children,
    label,
    clickPadding = false,
    ...rest
  }: Props = $props();

  const recipe = cva({
    base: {
      position: 'relative',
      appearance: 'none',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      flexShrink: 0,
      borderWidth: '1px',
      borderColor: 'border.emphasis',
      borderRadius: '4px',
      backgroundColor: 'surface.inset',
      transition: 'common',
      cursor: 'pointer',
      _hover: {
        backgroundColor: 'surface.hover',
      },
      _disabled: {
        cursor: 'not-allowed',
        _after: {
          opacity: '0',
        },
      },
      _checked: {
        _after: {
          opacity: '100',
        },
      },
      _after: {
        content: '""',
        position: 'absolute',
        inset: '0',
        display: 'block',
        maskImage:
          '[url(data:image/svg+xml;base64,PHN2ZyB4bWxucz0naHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmcnIHZpZXdCb3g9JzAgMCAyNCAyNCcgZmlsbD0nbm9uZScgc3Ryb2tlPSd3aGl0ZScgc3Ryb2tlLXdpZHRoPSczJyBzdHJva2UtbGluZWNhcD0ncm91bmQnIHN0cm9rZS1saW5lam9pbj0ncm91bmQnPjxwb2x5bGluZSBwb2ludHM9JzIwIDYgOSAxNyA0IDEyJz48L3BvbHlsaW5lPjwvc3ZnPg)]',
        maskRepeat: 'no-repeat',
        maskPosition: 'center',
        backgroundColor: 'surface.default',
        opacity: '0',
        transition: 'common',
      },
    },
    variants: {
      variant: {
        brand: {
          _checked: {
            borderColor: 'accent.default',
            backgroundColor: 'accent.default',
            _hover: {
              backgroundColor: 'accent.default',
            },
          },
        },
        info: {
          _checked: {
            borderColor: 'accent.default',
            backgroundColor: 'accent.default',
            _hover: {
              backgroundColor: 'accent.default',
            },
          },
        },
      },
      size: {
        sm: { width: '16px', height: '16px', _after: { maskSize: '10px' } },
        md: { width: '18px', height: '18px', _after: { maskSize: '12px' } },
        lg: { width: '20px', height: '20px', _after: { maskSize: '14px' } },
      },
      clickPadding: {
        true: {
          _before: {
            content: '""',
            position: 'absolute',
            inset: '-8px',
            display: 'block',
          },
        },
      },
    },
  });

  const disabled = $derived(rest.disabled ?? false);

  const wrapperStyle = $derived(
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        cursor: 'pointer',
      },
      disabled && {
        cursor: 'not-allowed',
        opacity: '40',
      },
      style,
    ),
  );

  const labelRecipe = cva({
    base: {
      color: 'text.muted',
      userSelect: 'none',
    },
    variants: {
      size: {
        sm: { fontSize: '12px' },
        md: { fontSize: '14px' },
        lg: { fontSize: '16px' },
      },
    },
  });
</script>

<label class={wrapperStyle} for={rest['name'] || rest['id']}>
  <input class={recipe({ variant, size, clickPadding })} type="checkbox" bind:checked {...rest} />

  {#if label}
    <span class={labelRecipe({ size })}>{label}</span>
  {/if}

  {#if children}
    {@render children?.()}
  {/if}
</label>
