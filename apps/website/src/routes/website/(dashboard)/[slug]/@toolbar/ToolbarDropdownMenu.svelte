<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { setContext, tick } from 'svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    opened?: boolean;
    autoFocus?: boolean;
    onclose?: () => void;
    children: Snippet;
  };

  let { style, opened = true, autoFocus = true, onclose, children }: Props = $props();
  let containerElement: HTMLDivElement | undefined = $state();

  const close = () => {
    onclose?.();
  };

  setContext('close', close);

  $effect(() => {
    if (containerElement) {
      tick().then(() => {
        const activeItem = containerElement?.querySelector('[data-active="true"]') as HTMLElement;
        const firstItem = containerElement?.querySelector('button[type="button"]') as HTMLElement;
        const targetItem = activeItem || firstItem;

        if (targetItem) {
          targetItem.scrollIntoView({ block: 'nearest' });
          if (autoFocus) {
            targetItem.focus();
          }
        }
      });
    }
  });

  $effect(() => {
    if (opened) {
      return pushEscapeHandler(() => {
        close();
        return true;
      });
    }
  });

  const getMenuItems = () => {
    return containerElement?.querySelectorAll('button[type="button"]');
  };

  const onKeydown = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    const menuItems = getMenuItems();
    if (!menuItems || menuItems.length === 0) {
      return;
    }

    const focusInList = containerElement?.contains(target);
    if (!focusInList) {
      return;
    }

    const pos = [...menuItems].indexOf(target);

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = (menuItems[pos + 1] || menuItems[0]) as HTMLElement;
      next?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      // eslint-disable-next-line unicorn/prefer-at
      const prev = (menuItems[pos - 1] || menuItems[menuItems.length - 1]) as HTMLElement;
      prev?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      (target as HTMLButtonElement).click();
    }
  };
</script>

<svelte:window onkeydown={onKeydown} />

<div
  bind:this={containerElement}
  class={css(
    {
      display: 'flex',
      flexDirection: 'column',
      maxHeight: '400px',
      overflowY: 'auto',
      '& > button:not(:first-of-type)': { borderTopWidth: '1px', borderColor: 'border.subtle' },
    },
    style,
  )}
>
  {@render children()}
</div>
