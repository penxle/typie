<script lang="ts">
  import { css, cx, sva } from '@typie/styled-system/css';
  import { getContext } from 'svelte';
  import Icon from './Icon.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component, Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  type BaseProps = {
    style?: SystemStyleObject;
    icon?: Component;
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

  let { type = 'button', style, variant = 'default', disabled = false, icon, children, prefix, onclick, ...rest }: Props = $props();

  const element = $derived(type === 'link' ? 'a' : 'button');
  const properties = $derived(type === 'link' ? { 'aria-disabled': disabled } : { type, disabled });

  let close = getContext<undefined | (() => void)>('close');

  let focused = $state(false);

  const recipe = sva({
    slots: ['root', 'icon'],

    base: {
      root: {
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        borderRadius: '6px',
        marginX: '2px',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '13px',
        fontWeight: 'medium',
        textAlign: 'left',
        transition: 'common',
        _enabled: {
          _hover: { backgroundColor: 'surface.muted' },
        },
        _disabled: {
          color: 'text.disabled',
        },
      },
    },
    variants: {
      variant: {
        default: {
          root: {
            color: 'text.subtle',
          },
          icon: {
            color: 'text.faint',
            _groupHover: { color: 'text.subtle' },
          },
        },
        danger: {
          root: {
            color: 'text.subtle',
            _hover: { color: 'text.danger' },
          },
          icon: {
            color: 'text.faint',
            _groupHover: { color: 'text.danger' },
          },
        },
      },
    },
  });

  const styles = $derived(recipe.raw({ variant }));
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
  class={cx('group', css(styles.root, style))}
>
  {@render prefix?.()}
  {#if icon}
    <Icon style={styles.icon} {icon} size={14} />
  {/if}
  {@render children?.()}
</svelte:element>
