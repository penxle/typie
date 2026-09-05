<script lang="ts">
  import { css, cva, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import RingSpinner from './RingSpinner.svelte';
  import type { RecipeVariantProps, SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  type RecipeProps = RecipeVariantProps<typeof recipe>;
  type BaseProps = RecipeProps & {
    style?: SystemStyleObject;
    element?: HTMLElement;
    loading?: boolean;
    disabled?: boolean;
    children: Snippet;
  };

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
      transition: 'colors',
      transitionProperty: '[unset]',
      userSelect: 'none',
      pointerEvents: { _disabled: 'none', _loading: 'none' },
    },
    variants: {
      variant: {
        primary: {
          fontWeight: 'semibold',
          letterSpacing: '-0.01em',
          color: 'surface.default',
          backgroundColor: {
            base: 'accent.default',
            _hover: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
            _active: '[color-mix(in oklch, token(colors.accent.default) 80%, black)]',
            _pressed: '[color-mix(in oklch, token(colors.accent.default) 80%, black)]',
          },
          boxShadow: 'sm',
          _disabled: { opacity: '40' },
        },
        secondary: {
          fontWeight: 'medium',
          color: 'text.muted',
          backgroundColor: {
            base: 'surface.default',
            _hover: 'surface.hover',
            _active: 'surface.active',
            _pressed: 'surface.active',
          },
          borderWidth: '1px',
          borderColor: 'border.hairline',
          boxShadow: 'sm',
          _disabled: { opacity: '40' },
        },
        ghost: {
          fontWeight: 'medium',
          color: {
            base: 'text.muted',
            _hover: 'text.default',
            _active: 'text.default',
            _pressed: 'text.default',
          },
          backgroundColor: {
            base: 'transparent',
            _hover: 'surface.hover',
            _active: 'surface.active',
            _pressed: 'surface.active',
          },
          _disabled: { opacity: '40' },
        },
        danger: {
          fontWeight: 'semibold',
          letterSpacing: '-0.01em',
          color: 'text.on.danger',
          backgroundColor: {
            base: 'danger.default',
            _hover: '[color-mix(in oklch, token(colors.danger.default) 88%, black)]',
            _active: '[color-mix(in oklch, token(colors.danger.default) 80%, black)]',
            _pressed: '[color-mix(in oklch, token(colors.danger.default) 80%, black)]',
          },
          boxShadow: 'sm',
          _disabled: { opacity: '40' },
        },
      },
      size: {
        sm: { borderRadius: '6px', paddingX: '12px', height: '30px', fontSize: '12px' },
        md: { borderRadius: '6px', paddingX: '16px', height: '34px', fontSize: '13px' },
        lg: { borderRadius: '8px', paddingX: '20px', height: '38px', fontSize: '14px' },
      },
    },
  });

  const spinnerRecipe = cva({
    base: {
      height: '[1lh]',
    },
    variants: {
      color: {
        primary: { color: 'surface.default' },
        secondary: { color: 'text.muted' },
        ghost: { color: 'text.muted' },
        danger: { color: 'text.on.danger' },
      },
    },
  });
</script>

<svelte:element
  this={type === 'link' ? 'a' : 'button'}
  bind:this={element}
  class={cx('group', css(recipe.raw({ variant, size }), loading && { position: 'relative' }, style))}
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
