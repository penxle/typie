<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { scale } from 'svelte/transition';
  import { createFloatingActions, portal, registerFocusTrapContainer } from '../actions';
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
    matchTriggerWidth?: boolean;
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
    matchTriggerWidth = false,
    disabled = false,
    onopen,
    onclose,
    trigger,
    children,
  }: Props = $props();

  let triggerEl = $state<HTMLButtonElement>();
  let contentEl = $state<HTMLDivElement>();
  let hasBeenOpened = $state(false);

  $effect(() => {
    const trigger = triggerEl;
    const content = contentEl;
    if (!open || !trigger || !content) return;

    const host = trigger.closest<HTMLElement>('[data-focus-trap]');
    if (!host) return;

    return untrack(() => registerFocusTrapContainer(host, content));
  });

  const { anchor, floating } = createFloatingActions({
    placement,
    offset,
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
  aria-haspopup="dialog"
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
    class={css({ position: 'fixed', inset: '0', zIndex: 'tooltip', pointerEvents: 'auto' })}
    onclick={() => (open = false)}
    role="none"
    use:portal={(document.querySelector('.tooltip-container') as HTMLElement) ?? undefined}
  ></div>

  <div
    bind:this={contentEl}
    style:min-width={matchTriggerWidth && triggerEl ? `${triggerEl.offsetWidth}px` : undefined}
    class={css(
      {
        borderRadius: '8px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.default',
        boxShadow: 'lg',
        _dark: { borderWidth: '1px', borderColor: 'border.default' },
        zIndex: 'tooltip',
        pointerEvents: 'auto',
      },
      contentStyle,
    )}
    role="dialog"
    use:floating={{ appendTo: document.querySelector('.tooltip-container') as Element | null }}
    transition:scale={{ start: 0.95, duration: 150 }}
  >
    {@render children?.({ close })}
  </div>
{/if}
