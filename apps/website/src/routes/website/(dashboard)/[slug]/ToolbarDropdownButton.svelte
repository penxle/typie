<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
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
    chevron?: boolean;
    placement?: Placement;
    anchor: Snippet<[{ open: () => void }]>;
    floating: Snippet<[{ close: () => void }]>;
  };

  let { style, size, label, active = false, chevron = false, placement = 'bottom-start', anchor, floating }: Props = $props();

  const { anchor: anchorAction, floating: floatingAction } = createFloatingActions({
    placement,
    offset: 8,
    onClickOutside: () => {
      close();
    },
  });

  let opened = $state(false);

  const app = getAppContext();

  const open = () => {
    opened = true;
    app.state.toolbarActive = true;
  };

  const close = () => {
    opened = false;
    app.state.toolbarActive = false;
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
        gap: '2px',
        borderRadius: '6px',
        size: '40px',
        color: active ? 'brand.500' : undefined,
        _hover: { backgroundColor: 'gray.100' },
        _pressed: { backgroundColor: 'gray.100' },
      },
      style,
    )}
    aria-pressed={opened}
    onclick={open}
    type="button"
    use:anchorAction
  >
    {@render anchor({ open })}
    <span class={css({ fontSize: '10px' })}>{label}</span>
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
          borderRadius: '6px',
          paddingX: chevron ? '4px' : '0',
          width: chevron ? 'fit' : '24px',
          height: '24px',
          textAlign: 'left',
          color: active ? 'brand.500' : undefined,
          backgroundColor: 'gray.100',
          _hover: { backgroundColor: 'gray.200' },
          _pressed: { backgroundColor: 'gray.200' },
        },
        style,
      )}
      aria-label={label}
      aria-pressed={opened}
      onclick={open}
      type="button"
      use:anchorAction
    >
      {@render anchor({ open })}

      {#if chevron}
        <Icon style={css.raw({ color: 'gray.500' })} icon={opened ? ChevronUpIcon : ChevronDownIcon} size={16} />
      {/if}
    </button>
  </ToolbarTooltip>
{/if}

{#if opened}
  <div class={css({ backgroundColor: 'white', zIndex: '50' })} use:floatingAction>
    {@render floating({ close })}
  </div>
{/if}
