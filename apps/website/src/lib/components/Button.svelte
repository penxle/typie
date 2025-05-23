<script lang="ts">
  import { css, cva, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import RingSpinner from './RingSpinner.svelte';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
  import type { RecipeVariantProps, SystemStyleObject } from '$styled-system/types';

  type RecipeProps = RecipeVariantProps<typeof recipe>;
  type BaseProps = {
    style?: SystemStyleObject;
    element?: HTMLElement;
    gradient?: boolean;
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

  let {
    type = 'button',
    style,
    gradient = false,
    disabled = false,
    loading = false,
    variant = 'primary',
    size = 'md',
    element = $bindable(),
    children,
    ...rest
  }: Props = $props();

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
          fontWeight: 'bold',
          color: {
            _enabled: {
              base: 'white',
              _hover: 'white',
              _active: 'white',
              _pressed: 'white',
            },
            _disabled: 'gray.500',
          },
          backgroundColor: {
            _enabled: {
              base: 'brand.500',
              _hover: 'brand.600',
              _active: 'brand.700',
              _pressed: 'brand.700',
            },
            _disabled: 'gray.200',
          },
        },
        secondary: {
          fontWeight: 'semibold',
          color: {
            _enabled: {
              base: 'gray.700',
              _hover: 'gray.700',
              _active: 'gray.700',
              _pressed: 'gray.700',
            },
            _disabled: 'gray.500',
          },
          backgroundColor: {
            _enabled: {
              base: 'white',
              _hover: 'gray.100',
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
        danger: {
          color: {
            _enabled: {
              base: 'white',
              _hover: 'white',
              _active: 'white',
              _pressed: 'white',
            },
            _disabled: 'gray.500',
          },
          backgroundColor: {
            _enabled: {
              base: 'red.600',
              _hover: 'red.500',
              _active: 'red.700',
              _pressed: 'red.700',
            },
            _disabled: 'gray.200',
          },
        },
      },
      size: {
        sm: { borderRadius: '4px', paddingX: '12px', height: '32px', fontSize: '13px' },
        md: { borderRadius: '6px', paddingX: '20px', height: '36px', fontSize: '14px' },
        lg: { borderRadius: '8px', paddingX: '28px', height: '40px', fontSize: '15px' },
      },
    },
  });

  const spinnerRecipe = cva({
    base: {
      height: '[1lh]',
    },
    variants: {
      color: {
        primary: { color: 'white' },
        secondary: { color: 'gray.700' },
        danger: { color: 'red.700' },
      },
    },
  });

  const gradientRecipe = cva({
    base: {
      position: 'absolute',
      inset: '0',
      bgGradient: 'to-br',
      gradientFrom: 'white/20',
      gradientTo: 'transparent',
      pointerEvents: 'none',
    },
    variants: {
      size: {
        sm: { borderRadius: '4px' },
        md: { borderRadius: '6px' },
        lg: { borderRadius: '8px' },
      },
    },
  });
</script>

<svelte:element
  this={type === 'link' ? 'a' : 'button'}
  bind:this={element}
  class={cx('group', css(recipe.raw({ variant, size }), (loading || gradient) && { position: 'relative' }, style))}
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

  {#if gradient && !disabled}
    <div class={gradientRecipe({ size })}></div>
  {/if}
</svelte:element>
