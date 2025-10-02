<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { scale } from 'svelte/transition';
  import { createFloatingActions } from '../actions';
  import { pushEscapeHandler } from '../utils';
  import type { OffsetOptions, Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    open?: boolean;
    placement?: Placement;
    offset?: OffsetOptions;
    style?: SystemStyleObject;
    contentStyle?: SystemStyleObject;
    disabled?: boolean;
    onopen?: () => void;
    onclose?: () => void;
    trigger: Snippet<[{ open: boolean }]>;
    children: Snippet<[{ close: () => void }]>;
  };

  let {
    open = $bindable(false),
    placement = 'top',
    offset = 8,
    style,
    contentStyle,
    disabled = false,
    onopen,
    onclose,
    trigger,
    children,
  }: Props = $props();

  let triggerEl = $state<HTMLButtonElement>();
  let contentEl = $state<HTMLDivElement>();
  let hasBeenOpened = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement,
    offset,
    onClickOutside: () => {
      open = false;
    },
  });

  const close = () => {
    open = false;
    triggerEl?.focus();
  };

  $effect(() => {
    if (!open) {
      if (hasBeenOpened) {
        onclose?.();
      }
      return;
    }

    hasBeenOpened = true;
  });

  $effect(() => {
    if (open) {
      return pushEscapeHandler(() => {
        if (open) {
          close();
          return true;
        }
        return false;
      });
    }
  });
</script>

<button
  bind:this={triggerEl}
  class={css(style)}
  aria-disabled={disabled}
  aria-expanded={open}
  {disabled}
  onclick={(e) => {
    if (disabled) {
      return;
    }
    e.preventDefault();
    open = !open;
    if (open) {
      onopen?.();
    }
  }}
  tabindex={disabled ? -1 : 0}
  type="button"
  use:anchor
>
  {@render trigger?.({ open })}
</button>

{#if open}
  <div
    bind:this={contentEl}
    class={css(
      {
        borderWidth: '1px',
        borderRadius: '8px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        zIndex: 'tooltip',
        pointerEvents: 'auto',
      },
      contentStyle,
    )}
    role="dialog"
    use:floating={{ appendTo: document.querySelector('.tooltip-container') as Element | null }}
    transition:scale={{ start: 0.9, duration: 200 }}
  >
    {@render children?.({ close })}
  </div>
{/if}
