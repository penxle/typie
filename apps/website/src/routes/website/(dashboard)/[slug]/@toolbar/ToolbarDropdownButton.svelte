<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import type { Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    size: 'large' | 'medium' | 'small';
    keys?: TooltipParameter['keys'];
    label: string;
    active?: boolean;
    disabled?: boolean;
    chevron?: boolean;
    placement?: Placement;
    opened?: boolean;
    onOpenChange?: (opened: boolean) => void;
    anchor: Snippet<[{ open: () => void; opened: boolean }]>;
    floating: Snippet<[{ close: () => void }]>;
  };

  let {
    style,
    size,
    keys,
    label,
    active = false,
    disabled = false,
    chevron = false,
    placement = 'bottom',
    opened: externalOpened,
    onOpenChange,
    anchor,
    floating,
  }: Props = $props();

  const { anchor: anchorAction, floating: floatingAction } = createFloatingActions({
    placement,
    offset: 8,
    onClickOutside: () => {
      close();
    },
  });

  let opened = $state(false);

  $effect(() => {
    if (externalOpened === undefined) return;

    if (externalOpened && !opened) {
      open();
    } else if (!externalOpened && opened) {
      close();
    }
  });

  const open = () => {
    opened = true;
    onOpenChange?.(true);
  };

  const close = () => {
    opened = false;
    onOpenChange?.(false);
  };
</script>

{#if size === 'large'}
  <button
    class={css(
      {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        flexDirection: 'column',
        gap: '4px',
        borderRadius: '4px',
        size: '48px',
        color: active ? 'text.brand' : 'text.subtle',
        transition: 'common',
        _enabled: {
          _hover: { color: 'text.brand' },
          _expanded: { color: 'text.brand' },
        },
        _disabled: { opacity: '50' },
        flexShrink: '0',
      },
      style,
    )}
    aria-expanded={opened}
    aria-haspopup="menu"
    {disabled}
    onclick={open}
    type="button"
    use:anchorAction
  >
    {@render anchor({ open, opened })}
    <span class={css({ fontSize: '11px' })}>{label}</span>
  </button>
{:else if size === 'medium'}
  <button
    class={css({
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      gap: '2px',
      borderRadius: '4px',
      paddingX: chevron ? '4px' : '0',
      width: chevron ? 'fit' : '28px',
      height: '28px',
      textAlign: 'left',
      color: active ? 'text.brand' : 'text.subtle',
      transition: 'common',
      _enabled: {
        _hover: { color: 'text.brand' },
        _expanded: { color: 'text.brand' },
      },
      _disabled: { opacity: '50' },
      flexShrink: '0',
    })}
    aria-expanded={opened}
    aria-haspopup="menu"
    aria-label={label}
    {disabled}
    onclick={open}
    type="button"
    use:anchorAction
    use:tooltip={{ message: label, keys, delay: 200, arrow: false }}
  >
    {@render anchor({ open, opened })}

    {#if chevron}
      <Icon
        style={css.raw({
          color: 'text.faint',
          transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
          transitionDuration: '150ms',
        })}
        icon={ChevronDownIcon}
        size={16}
      />
    {/if}
  </button>
{:else if size === 'small'}
  <button
    class={css(
      {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        gap: '2px',
        borderRadius: '4px',
        paddingX: chevron ? '4px' : '0',
        width: chevron ? 'fit' : '24px',
        height: '24px',
        textAlign: 'left',
        color: active ? 'text.brand' : 'text.subtle',
        transition: 'common',
        _enabled: {
          _hover: { color: 'text.brand' },
          _expanded: { color: 'text.brand' },
        },
        _disabled: { opacity: '50' },
        flexShrink: '0',
      },
      style,
    )}
    aria-expanded={opened}
    aria-haspopup="menu"
    aria-label={label}
    {disabled}
    onclick={open}
    type="button"
    use:anchorAction
    use:tooltip={{ message: label, keys, delay: 1000, arrow: false }}
  >
    {@render anchor({ open, opened })}

    {#if chevron}
      <Icon
        style={css.raw({
          color: 'text.faint',
          transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
          transitionDuration: '150ms',
        })}
        icon={ChevronDownIcon}
        size={16}
      />
    {/if}
  </button>
{/if}

{#if opened}
  <div
    class={css({
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderBottomRadius: '4px',
      backgroundColor: 'surface.default',
      zIndex: 'overEditor',
      boxShadow: 'small',
      overflow: 'hidden',
    })}
    use:floatingAction
    in:fly={{ y: -5, duration: 150 }}
  >
    {@render floating({ close })}
  </div>
{/if}
