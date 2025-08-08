<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { setContext, tick } from 'svelte';
  import { scale } from 'svelte/transition';
  import { afterNavigate } from '$app/navigation';
  import { createFloatingActions, focusTrap, portal } from '../actions';
  import type { OffsetOptions, Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    open?: boolean;
    placement?: Placement;
    offset?: OffsetOptions;
    style?: SystemStyleObject;
    listStyle?: SystemStyleObject;
    setFullWidth?: boolean;
    disableAutoUpdate?: boolean;
    onopen?: () => void;
    onclose?: () => void;
    button?: Snippet<[{ open: boolean }]>;
    action?: Snippet;
    children?: Snippet<[{ close: () => void }]>;
    contextMenuPosition?: { x: number; y: number } | null;
  };

  let {
    open = $bindable(false),
    placement = 'bottom',
    offset = 6,
    style,
    listStyle,
    setFullWidth = false,
    disableAutoUpdate = false,
    onopen,
    onclose,
    button,
    action,
    children,
    contextMenuPosition = null,
  }: Props = $props();

  let buttonEl = $state<HTMLButtonElement>();
  let menuEl = $state<HTMLUListElement>();

  const { anchor, floating } = createFloatingActions({
    placement,
    offset,
    disableAutoUpdate,
    onClickOutside: () => {
      open = false;
    },
  });

  const close = () => {
    open = false;
    buttonEl?.focus();
  };

  setContext('close', close);

  afterNavigate(() => {
    open = false;
  });

  $effect(() => {
    if (!open && onclose) {
      onclose();
    }
  });

  const getMenuItems = () => {
    return menuEl?.querySelectorAll('[role="menuitem"], [role="menuitemradio"]');
  };

  const onKeydown = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    if (open) {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
        return;
      }

      if (e.key === 'Tab') {
        close();
        return;
      }

      const focusInList = menuEl?.contains(target);

      const menuItems = getMenuItems();
      if (!menuItems || menuItems.length === 0) {
        return;
      }

      // eslint-disable-next-line unicorn/prefer-spread
      const pos = Array.from(menuItems).indexOf(target);

      if (focusInList) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          const next = (menuItems[pos + 1] || menuItems[0]) as HTMLElement;
          next?.focus();
        }

        if (e.key === 'ArrowUp') {
          e.preventDefault();
          // eslint-disable-next-line unicorn/prefer-at
          const prev = (menuItems[pos - 1] || menuItems[menuItems.length - 1]) as HTMLElement;
          prev?.focus();
        }
      } else {
        if (['ArrowDown', 'ArrowUp'].includes(e.key)) {
          e.preventDefault();
          (menuItems[0] as HTMLElement).focus();
        }
      }
    } else {
      // 버튼에 포커스가 있을 때 아래 키로 메뉴를 열고 첫번째 항목에 포커스
      const focusInButton = buttonEl?.contains(target);
      if (focusInButton && e.key === 'ArrowDown') {
        e.preventDefault();
        open = true;
        tick().then(() => {
          const menuItems = getMenuItems();
          if (!menuItems || menuItems.length === 0) {
            return;
          }
          (menuItems[0] as HTMLElement).focus();
        });
      }
    }
  };
</script>

<svelte:window onkeydown={onKeydown} />

{#if contextMenuPosition}
  <div
    style:left={`${contextMenuPosition.x}px`}
    style:top={`${contextMenuPosition.y}px`}
    class={css({
      position: 'fixed',
      size: '0',
      pointerEvents: 'none',
    })}
    use:anchor
  ></div>
{:else if button}
  <button
    bind:this={buttonEl}
    class={css(style)}
    aria-expanded={open}
    onclick={(e) => {
      e.preventDefault();
      open = !open;
      if (open) {
        onopen?.();
      }
    }}
    type="button"
    use:anchor
  >
    {@render button?.({ open })}
  </button>
{/if}

{#if open}
  <div class={css({ position: 'fixed', inset: '0', zIndex: 'menu' })} onclick={close} role="none" use:portal></div>

  <ul
    bind:this={menuEl}
    style:width={setFullWidth ? `${buttonEl?.getBoundingClientRect().width}px` : undefined}
    class={css(
      {
        display: 'flex',
        flexDirection: 'column',
        gap: '2px',
        borderWidth: '1px',
        borderRadius: '8px',
        paddingY: '2px',
        minWidth: '160px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        overflowY: 'auto',
        zIndex: 'menu',
      },
      action && { paddingBottom: '0' },
      listStyle,
    )}
    role="menu"
    use:floating
    use:focusTrap={{ fallbackFocus: menuEl, escapeDeactivates: false, allowOutsideClick: true }}
    transition:scale={{ start: 0.95, duration: 150 }}
  >
    {#if action}
      <li>
        <ul class={css({ display: 'flex', flexDirection: 'column', gap: '4px', overflowY: 'auto' })}>
          {@render children?.({ close })}
        </ul>
      </li>
    {:else}
      {@render children?.({ close })}
    {/if}

    {#if action}
      <li class={css({ position: 'sticky', bottom: '0', paddingBottom: '12px', backgroundColor: 'surface.default' })}>
        {@render action?.()}
      </li>
    {/if}
  </ul>
{/if}
