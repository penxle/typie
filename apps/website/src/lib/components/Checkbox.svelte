<script lang="ts">
  import { css, cva } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';
  import type { RecipeVariantProps } from '$styled-system/css';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    checked?: boolean;
    children?: Snippet;
    label?: string;
  } & Omit<HTMLInputAttributes, 'size' | 'style'> &
    RecipeVariantProps<typeof recipe>;

  let { size = 'md', style, checked = $bindable(false), children, label, ...rest }: Props = $props();

  const recipe = cva({
    base: {
      position: 'relative',
      appearance: 'none',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      flexShrink: 0,
      borderWidth: '1px',
      borderColor: 'border.strong',
      borderRadius: '4px',
      backgroundColor: 'surface.default',
      transition: 'common',
      cursor: 'pointer',
      outline: 'none',
      _disabled: {
        backgroundColor: 'surface.muted!',
        borderColor: 'gray.300!',
        cursor: 'not-allowed',
        _after: {
          opacity: '0',
        },
      },
      _checked: {
        borderColor: 'brand.500',
        backgroundColor: 'accent.brand.default',
        _after: {
          opacity: '100',
        },
      },
      _after: {
        content: '""',
        position: 'absolute',
        inset: '0',
        display: 'block',
        backgroundImage:
          '[url(data:image/svg+xml;base64,PHN2ZyB4bWxucz0naHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmcnIHZpZXdCb3g9JzAgMCAyNCAyNCcgZmlsbD0nbm9uZScgc3Ryb2tlPSd3aGl0ZScgc3Ryb2tlLXdpZHRoPSczJyBzdHJva2UtbGluZWNhcD0ncm91bmQnIHN0cm9rZS1saW5lam9pbj0ncm91bmQnPjxwb2x5bGluZSBwb2ludHM9JzIwIDYgOSAxNyA0IDEyJz48L3BvbHlsaW5lPjwvc3ZnPg)]',
        backgroundRepeat: 'no-repeat',
        backgroundPosition: 'center',
        opacity: '0',
        transition: 'common',
      },
    },
    variants: {
      size: {
        sm: { width: '16px', height: '16px', _after: { backgroundSize: '10px' } },
        md: { width: '18px', height: '18px', _after: { backgroundSize: '12px' } },
        lg: { width: '20px', height: '20px', _after: { backgroundSize: '14px' } },
      },
    },
  });

  const wrapperStyle = css(
    {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      cursor: 'pointer',
    },
    style,
  );

  const labelRecipe = cva({
    base: {
      color: 'text.subtle',
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
  <input class={recipe({ size })} type="checkbox" bind:checked {...rest} />

  {#if label}
    <span class={labelRecipe({ size })}>{label}</span>
  {/if}

  {#if children}
    {@render children?.()}
  {/if}
</label>
