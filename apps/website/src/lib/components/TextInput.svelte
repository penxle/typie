<script lang="ts">
  import { css, cva } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Icon from './Icon.svelte';
  import type { Component, Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';
  import type { RecipeVariantProps, SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    element?: HTMLInputElement;
    name?: string;
    leftIcon?: Component;
    rightIcon?: Component;
    hidden?: boolean;
    leftItem?: Snippet;
    rightItem?: Snippet;
    autofocus?: boolean;
  } & RecipeVariantProps<typeof recipe> &
    Omit<HTMLInputAttributes, 'class' | 'style' | 'size' | 'name' | 'autofocus'>;

  let {
    name,
    value = $bindable(),
    style,
    size = 'md',
    element = $bindable(),
    leftIcon,
    rightIcon,
    hidden = false,
    leftItem,
    rightItem,
    autofocus = false,
    ...rest
  }: Props = $props();

  $effect(() => {
    if (autofocus) {
      setTimeout(() => {
        element?.focus();
      });
    }
  });

  const recipe = cva({
    base: {
      display: 'flex',
      alignItems: 'center',
      borderWidth: '1px',
      color: 'gray.500',
      backgroundColor: 'white',
      transition: 'common',
      _hover: {
        borderColor: 'brand.400',
      },
      '&:has(input:focus)': {
        borderColor: 'brand.600',
      },
      '&:has(input:not(:placeholder-shown)), &:has(input[aria-live="polite"])': {
        color: 'gray.950',
        borderColor: 'gray.300',
      },
      '&:has(input:disabled)': {
        color: 'gray.500',
        backgroundColor: 'gray.200',
        borderColor: 'gray.200',
      },
      '&:has(input:read-only)': {
        color: 'gray.500',
        backgroundColor: 'gray.200',
        borderColor: 'gray.200',
      },
      '&:has(input[aria-invalid="true"])': {
        borderColor: 'red.600',
        '&:has(input:focus)': {
          borderColor: 'red.600',
        },
        '&:has(input:not(:placeholder-shown)), &:has(input[aria-live="polite"])': {
          color: 'gray.950',
          backgroundColor: 'white',
        },
      },
    },
    variants: {
      size: {
        sm: {
          borderRadius: '4px',
          paddingX: '12px',
          height: '32px',
          fontSize: '13px',
        },
        md: {
          borderRadius: '6px',
          paddingX: '12px',
          height: '38px',
          fontSize: '14px',
        },
        lg: {
          borderRadius: '8px',
          paddingX: '16px',
          height: '44px',
          fontSize: '15px',
        },
      },
    },
  });
</script>

<label class={css(recipe.raw({ size }), style)} for={name} {hidden}>
  {#if leftIcon}
    <div class={flex({ align: 'center', marginRight: '8px' })}>
      <Icon icon={leftIcon} size={18} />
    </div>
  {/if}

  {#if leftItem}
    <div class={css({ marginRight: '8px' })}>
      {@render leftItem()}
    </div>
  {/if}

  <input
    bind:this={element}
    id={name}
    {name}
    class={css({ flexGrow: '1', width: 'full', minWidth: '0' })}
    type="text"
    bind:value
    {...rest}
    aria-live={value ? 'polite' : 'off'}
  />

  {#if rightIcon}
    <div class={flex({ align: 'center', marginLeft: '20px' })}>
      <Icon icon={rightIcon} size={18} />
    </div>
  {/if}

  {#if rightItem}
    <div class={css({ marginLeft: '8px' })}>
      {@render rightItem()}
    </div>
  {/if}
</label>
