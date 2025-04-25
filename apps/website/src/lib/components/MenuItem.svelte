<script lang="ts">
  import { getContext } from 'svelte';
  import { css, cva, cx } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
  import type { SystemStyleObject } from '$styled-system/types';

  type BaseProps = {
    style?: SystemStyleObject;
    disabled?: boolean;
    variant?: 'default' | 'danger';
    children?: Snippet;
    prefix?: Snippet;
    onclick?: () => void;
  };

  type ButtonAttributes = Omit<HTMLButtonAttributes, 'type' | 'style' | 'disabled' | 'prefix'>;
  type ButtonProps = ButtonAttributes & {
    type?: 'button';
  };

  type LinkAttributes = Omit<HTMLAnchorAttributes, 'type' | 'style' | 'disabled' | 'prefix'>;
  type LinkProps = LinkAttributes & {
    type?: 'link';
    external?: boolean;
  };

  type ButtonAllProps = BaseProps & ButtonProps;
  type LinkAllProps = BaseProps & LinkProps;

  type Props = ButtonAllProps | LinkAllProps;

  let { type = 'button', style, variant = 'default', disabled = false, children, prefix, onclick, ...rest }: Props = $props();

  const element = $derived(type === 'link' ? 'a' : 'button');
  const properties = $derived(type === 'link' ? { 'aria-disabled': disabled } : { type, disabled });

  let close = getContext<undefined | (() => void)>('close');

  let focused = $state(false);
</script>

<svelte:element
  this={element}
  onblur={() => (focused = false)}
  onclick={() => {
    close?.();
    onclick?.();
  }}
  onfocus={() => (focused = true)}
  role="menuitem"
  tabindex={focused ? 0 : -1}
  {...type === 'link' && 'external' in rest && rest.external ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
  {...properties}
  {...rest}
  class={cx(
    cva({
      base: flex.raw({
        alignItems: 'center',
        gap: '10px',
        borderRadius: '4px',
        marginX: '4px',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '13px',
        fontWeight: 'medium',
        textAlign: 'left',
        _enabled: {
          _hover: {
            backgroundColor: 'gray.100',
          },
          _focus: {
            backgroundColor: 'gray.200',
          },
          _active: {
            backgroundColor: 'gray.100',
          },
          _selected: {
            backgroundColor: 'gray.100',
          },
        },
        _disabled: {
          color: 'gray.300',
        },
      }),
      variants: {
        variant: {
          default: {
            color: 'gray.700',
          },
          danger: {
            color: 'red.600',
          },
        },
      },
    })({ variant }),
    css(style),
  )}
>
  {@render prefix?.()}
  {@render children?.()}
</svelte:element>
