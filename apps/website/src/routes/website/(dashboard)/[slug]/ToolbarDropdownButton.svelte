<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import ToolbarTooltip from './ToolbarTooltip.svelte';
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';

  type Props = {
    size: 'large' | 'small';
    label: string;
    chevron?: boolean;
    placement?: Placement;
    anchor: Snippet<[{ open: () => void }]>;
    floating: Snippet<[{ close: () => void }]>;
  };

  let { size, label, chevron = false, placement = 'bottom-start', anchor, floating }: Props = $props();

  const { anchor: anchorAction, floating: floatingAction } = createFloatingActions({
    placement,
    offset: 8,
    onClickOutside: () => {
      opened = false;
    },
  });

  let opened = $state(false);

  const open = () => (opened = true);
  const close = () => (opened = false);
</script>

{#if size === 'large'}
  <button
    class={center({
      flexDirection: 'column',
      gap: '4px',
      borderRadius: '4px',
      size: '54px',
      _hover: { backgroundColor: 'gray.100' },
      _pressed: { backgroundColor: 'gray.200' },
    })}
    aria-pressed={opened}
    onclick={open}
    type="button"
    use:anchorAction
  >
    {@render anchor({ open })}
    <span class={css({ fontSize: '11px' })}>{label}</span>
  </button>
{:else if size === 'small'}
  <ToolbarTooltip {label}>
    <button
      class={center({
        gap: '4px',
        borderRadius: '4px',
        paddingX: chevron ? '4px' : '0',
        width: chevron ? 'fit' : '24px',
        height: '24px',
        _hover: { backgroundColor: 'gray.100' },
        _pressed: { backgroundColor: 'gray.200' },
      })}
      aria-label={label}
      aria-pressed={opened}
      onclick={open}
      type="button"
      use:anchorAction
    >
      {@render anchor({ open })}

      {#if chevron}
        <Icon icon={opened ? ChevronUpIcon : ChevronDownIcon} size={12} />
      {/if}
    </button>
  </ToolbarTooltip>
{/if}

{#if opened}
  <div class={css({ backgroundColor: 'white' })} use:floatingAction>
    {@render floating({ close })}
  </div>
{/if}
