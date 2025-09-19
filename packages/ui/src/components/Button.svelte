<script lang="ts">
  import { css, cva, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import RingSpinner from './RingSpinner.svelte';
  import type { RecipeVariantProps, SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

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
      transitionProperty: '[unset]',
      userSelect: 'none',
      pointerEvents: { _disabled: 'none', _loading: 'none' },
    },
    variants: {
      variant: {
        primary: {
          fontWeight: 'bold',
          color: {
            _enabled: {
              base: 'text.bright',
              _hover: 'text.bright',
              _active: 'text.bright',
              _pressed: 'text.bright',
            },
            _disabled: 'text.disabled',
          },
          backgroundColor: {
            _enabled: {
              base: 'accent.brand.default',
              _hover: 'accent.brand.hover',
              _active: 'accent.brand.active',
              _pressed: 'accent.brand.active',
            },
            _disabled: 'interactive.disabled',
          },
        },
        secondary: {
          fontWeight: 'semibold',
          color: {
            _enabled: {
              base: 'text.subtle',
              _hover: 'text.subtle',
              _active: 'text.subtle',
              _pressed: 'text.subtle',
            },
            _disabled: 'text.disabled',
          },
          backgroundColor: {
            _enabled: {
              base: 'surface.default',
              _hover: 'surface.subtle',
              _active: 'interactive.hover',
              _pressed: 'interactive.hover',
            },
            _disabled: 'interactive.disabled',
          },
          borderWidth: '1px',
          borderColor: {
            _enabled: 'border.strong',
            _disabled: 'border.default',
          },
        },
        danger: {
          color: {
            _enabled: {
              base: 'text.bright',
              _hover: 'text.bright',
              _active: 'text.bright',
              _pressed: 'text.bright',
            },
            _disabled: 'text.disabled',
          },
          backgroundColor: {
            _enabled: {
              base: 'accent.danger.default',
              _hover: 'accent.danger.hover',
              _active: 'accent.danger.active',
              _pressed: 'accent.danger.active',
            },
            _disabled: 'interactive.disabled',
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
        primary: { color: 'text.bright' },
        secondary: { color: 'text.subtle' },
        danger: { color: 'text.danger' },
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
