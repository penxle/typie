<script lang="ts">
  import { css, cva } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import RingSpinner from './RingSpinner.svelte';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
  import type { RecipeVariantProps, SystemStyleObject } from '$styled-system/types';

  type RecipeProps = RecipeVariantProps<typeof recipe>;
  type BaseProps = {
    style?: SystemStyleObject;
    loading?: boolean;
    disabled?: boolean;
    children: Snippet;
  } & RecipeProps;

  type ButtonAttributes = Omit<HTMLButtonAttributes, 'type' | 'class' | 'style' | 'disabled'>;
  type ButtonProps = ButtonAttributes & {
    type?: 'button' | 'reset' | 'submit';
  };

  type LinkAttributes = Omit<HTMLAnchorAttributes, 'type' | 'class' | 'style'>;
  type LinkProps = LinkAttributes & {
    type?: 'link';
    external?: boolean;
  };

  type ButtonAllProps = BaseProps & ButtonProps;
  type LinkAllProps = BaseProps & LinkProps;

  type Props = ButtonAllProps | LinkAllProps;

  let { type = 'button', style, disabled = false, loading = false, variant = 'primary', size = 'md', children, ...rest }: Props = $props();

  const recipe = cva({
    base: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      textAlign: 'center',
      outlineOffset: '0',
      transition: 'colors',
      transitionProperty: 'unset',
      userSelect: 'none',
      pointerEvents: { _disabled: 'none', _loading: 'none' },
    },
    variants: {
      variant: {
        primary: {
          color: {
            _enabled: {
              base: 'white',
              _hover: 'white',
              _focusVisible: 'white',
              _active: 'white',
              _pressed: 'white',
            },
            _disabled: 'gray.500',
          },
          backgroundColor: {
            _enabled: {
              base: 'brand.500',
              _hover: 'brand.600',
              _focusVisible: 'brand.600',
              _active: 'brand.700',
              _pressed: 'brand.700',
            },
            _disabled: 'gray.200',
          },
        },
        secondary: {
          color: {
            _enabled: {
              base: 'gray.700',
              _hover: 'gray.700',
              _focusVisible: 'gray.700',
              _active: 'gray.700',
              _pressed: 'gray.700',
            },
            _disabled: 'gray.500',
          },
          backgroundColor: {
            _enabled: {
              base: 'white',
              _hover: 'gray.100',
              _focusVisible: 'gray.100',
              _active: 'gray.300',
              _pressed: 'gray.300',
            },
            _disabled: 'gray.200',
          },
          borderWidth: '1px',
          borderColor: {
            _enabled: 'gray.300',
            _disabled: 'gray.200',
          },
        },
      },
      size: {
        sm: { borderRadius: '6px', paddingX: '14px', paddingY: '6px', height: '32px', fontSize: '14px', fontWeight: 'semibold' },
        md: { borderRadius: '8px', paddingX: '20px', paddingY: '10px', height: '40px', fontSize: '14px', fontWeight: 'semibold' },
        lg: { borderRadius: '10px', paddingX: '30px', paddingY: '10px', height: '48px', fontSize: '16px', fontWeight: 'semibold' },
      },
    },
  });

  const spinnerRecipe = cva({
    base: {
      height: 'full',
    },
    variants: {
      color: {
        primary: { color: 'white' },
        secondary: { color: 'gray.700' },
      },
    },
  });
</script>

<svelte:element
  this={type === 'link' ? 'a' : 'button'}
  class={css(recipe.raw({ variant, size }), loading && { position: 'relative' }, style)}
  aria-busy={loading}
  role="button"
  tabindex="0"
  {...type === 'link' && 'external' in rest && rest.external ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
  {...type === 'link' ? { 'aria-disabled': disabled } : { type, disabled }}
  {...rest}
>
  {#if loading}
    <div class={center({ position: 'absolute', inset: '0', padding: '[inherit]' })}>
      <RingSpinner style={spinnerRecipe.raw({ color: variant })} />
    </div>
  {/if}

  <div class={css({ display: 'contents' }, loading && { visibility: 'hidden' })}>
    {@render children()}
  </div>
</svelte:element>
