<script lang="ts">
  import { css, cx, sva } from '@typie/styled-system/css';
  import { getContext } from 'svelte';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import Icon from './Icon.svelte';
  import RingSpinner from './RingSpinner.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component, Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  type BaseProps = {
    style?: SystemStyleObject;
    icon?: Component;
    disabled?: boolean;
    loading?: boolean;
    variant?: 'default' | 'danger';
    children?: Snippet;
    prefix?: Snippet;
    suffix?: Snippet;
    onclick?: () => void;
    noCloseOnClick?: boolean;
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

  let {
    type = 'button',
    style,
    variant = 'default',
    disabled = false,
    loading = false,
    icon,
    children,
    prefix,
    suffix,
    onclick,
    noCloseOnClick = false,
    ...rest
  }: Props = $props();

  const element = $derived(type === 'link' ? 'a' : 'button');
  const properties = $derived(type === 'link' ? { 'aria-disabled': disabled || loading } : { type, disabled: disabled || loading });

  let close = getContext<undefined | (() => void)>('close');

  // In focus-managed menus hover moves focus, so hover itself must not paint a second highlight.
  const focusManaged = getContext<boolean>('menuFocusManaged') ?? false;

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
        outlineWidth: '0',
        transition: 'common',
        _focus: { backgroundColor: 'surface.hover' },
        _disabled: {
          opacity: '40',
          backgroundColor: 'transparent!',
          pointerEvents: 'none',
        },
        '[data-submenu-safezone] &': {
          cursor: 'default',
        },
      },
    },
    variants: {
      variant: {
        default: {
          root: {
            color: 'text.muted',
            _focus: { color: 'text.default' },
          },
          icon: {
            color: 'text.muted',
            _groupFocus: { color: 'text.default' },
          },
        },
        danger: {
          root: {
            color: 'text.muted',
            _focus: { color: 'danger.default' },
          },
          icon: {
            color: 'text.muted',
            _groupFocus: { color: 'danger.default' },
          },
        },
      },
      focusManaged: {
        true: {},
        false: {
          root: {
            _hover: { backgroundColor: 'surface.hover' },
            '[data-submenu-safezone] &': { _hover: { backgroundColor: 'transparent' } },
          },
        },
      },
    },
    compoundVariants: [
      {
        variant: 'default',
        focusManaged: false,
        css: {
          root: {
            _hover: { color: 'text.default' },
            '[data-submenu-safezone] &': { _hover: { color: 'text.muted' } },
          },
          icon: {
            _groupHover: { color: 'text.default' },
            '[data-submenu-safezone] .group:hover &': { color: 'text.muted' },
          },
        },
      },
      {
        variant: 'danger',
        focusManaged: false,
        css: {
          root: {
            _hover: { color: 'danger.default' },
            '[data-submenu-safezone] &': { _hover: { color: 'text.muted' } },
          },
          icon: {
            _groupHover: { color: 'danger.default' },
            '[data-submenu-safezone] .group:hover &': { color: 'text.muted' },
          },
        },
      },
    ],
  });

  const styles = $derived(recipe.raw({ variant, focusManaged }));
</script>

<svelte:element
  this={element}
  onblur={() => (focused = false)}
  onclick={() => {
    if (!loading && !noCloseOnClick) {
      close?.();
    }
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
  {#if type === 'link' && 'external' in rest && rest.external}
    <Icon style={styles.icon} aria-label="새 탭에서 열기" icon={ExternalLinkIcon} size={12} />
  {/if}
  {#if loading}
    <RingSpinner style={css.raw({ size: '14px', marginLeft: 'auto' })} />
  {:else}
    {@render suffix?.()}
  {/if}
</svelte:element>
