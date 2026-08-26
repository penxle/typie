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
    keys?: TooltipParameter['keys'];
    label: string;
    active?: boolean;
    disabled?: boolean;
    chevron?: boolean;
    placement?: Placement;
    opened?: boolean;
    onOpenChange?: (opened: boolean) => void;
    onEscape?: () => void;
    anchor: Snippet<[{ open: () => void; opened: boolean }]>;
    floating: Snippet<[{ close: () => void; opened: boolean }]>;
  };

  let {
    style,
    keys,
    label,
    active = false,
    disabled = false,
    chevron = false,
    placement = 'bottom',
    opened: externalOpened,
    onOpenChange,
    onEscape,
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
    if (disabled) return;
    opened = true;
    onOpenChange?.(true);
  };

  const close = () => {
    opened = false;
    onOpenChange?.(false);
  };

  $effect(() => {
    if (disabled && opened) close();
  });

  const handleEscape = () => {
    close();
    onEscape?.();
  };
</script>

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
  use:tooltip={{ message: label, keys, arrow: false }}
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
    {@render floating({ close: handleEscape, opened })}
  </div>
{/if}
