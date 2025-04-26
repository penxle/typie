<script lang="ts">
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import ToolbarTooltip from './ToolbarTooltip.svelte';
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    size: 'large' | 'small';
    label: string;
    active?: boolean;
    disabled?: boolean;
    chevron?: boolean;
    placement?: Placement;
    anchor: Snippet<[{ open: () => void; opened: boolean }]>;
    floating: Snippet<[{ close: () => void }]>;
  };

  let { style, size, label, active = false, disabled = false, chevron = false, placement = 'bottom', anchor, floating }: Props = $props();

  const { anchor: anchorAction, floating: floatingAction } = createFloatingActions({
    placement,
    offset: 8,
    onClickOutside: () => {
      close();
    },
  });

  let opened = $state(false);

  const open = () => {
    opened = true;
  };

  const close = () => {
    opened = false;
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
        color: active ? 'brand.400' : 'gray.700',
        transition: 'common',
        _enabled: {
          _hover: { color: 'brand.400' },
          _pressed: { color: 'brand.400' },
        },
        _disabled: { opacity: '50' },
      },
      style,
    )}
    aria-pressed={opened}
    {disabled}
    onclick={open}
    type="button"
    use:anchorAction
  >
    {@render anchor({ open, opened })}
    <span class={css({ fontSize: '11px' })}>{label}</span>
  </button>
{:else if size === 'small'}
  <ToolbarTooltip {label}>
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
          color: active ? 'brand.400' : 'gray.700',
          transition: 'common',
          _enabled: {
            _hover: { color: 'brand.400' },
            _pressed: { color: 'brand.400' },
          },
          _disabled: { opacity: '50' },
        },
        style,
      )}
      aria-label={label}
      aria-pressed={opened}
      {disabled}
      onclick={open}
      type="button"
      use:anchorAction
    >
      {@render anchor({ open, opened })}

      {#if chevron}
        <Icon
          style={css.raw({
            color: 'gray.500',
            transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
            transitionDuration: '150ms',
          })}
          icon={ChevronDownIcon}
          size={16}
        />
      {/if}
    </button>
  </ToolbarTooltip>
{/if}

{#if opened}
  <div
    class={css({
      borderWidth: '1px',
      borderColor: 'gray.100',
      borderBottomRadius: '4px',
      backgroundColor: 'white',
      zIndex: '50',
      boxShadow: 'small',
      overflow: 'hidden',
    })}
    use:floatingAction
    in:fly={{ y: -5, duration: 150 }}
  >
    {@render floating({ close })}
  </div>
{/if}
