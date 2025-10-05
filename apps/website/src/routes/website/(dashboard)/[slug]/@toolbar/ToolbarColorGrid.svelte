<script generics="Item extends { label: string; value: string; color: string | null }" lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { tick } from 'svelte';

  type Props = {
    items: readonly Item[];
    currentValue: Item['value'];
    columns: number;
    shape?: 'circle' | 'square';
    showNone?: boolean;
    onSelect: (value: Item['value']) => void;
    onClose?: () => void;
    opened?: boolean;
  };

  let { items, currentValue, columns, shape = 'circle', showNone = false, onSelect, onClose, opened = true }: Props = $props();

  let containerElement: HTMLDivElement | undefined = $state();

  const close = () => {
    onClose?.();
  };

  $effect(() => {
    if (opened && containerElement) {
      tick().then(() => {
        const activeButton = containerElement?.querySelector('[data-active="true"]') as HTMLElement;
        const firstButton = containerElement?.querySelector('button[type="button"]') as HTMLElement;
        const targetButton = activeButton || firstButton;

        if (targetButton) {
          targetButton.focus();
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

  const getButtons = () => {
    return containerElement?.querySelectorAll('button[type="button"]');
  };

  const onKeydown = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    const buttons = getButtons();
    if (!buttons || buttons.length === 0) {
      return;
    }

    const focusInGrid = containerElement?.contains(target);
    if (!focusInGrid) {
      return;
    }

    const pos = [...buttons].indexOf(target);
    const row = Math.floor(pos / columns);
    const col = pos % columns;

    if (e.key === 'ArrowRight') {
      e.preventDefault();
      const nextPos = pos + 1 < buttons.length ? pos + 1 : 0;
      (buttons[nextPos] as HTMLElement).focus();
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      const prevPos = pos - 1 >= 0 ? pos - 1 : buttons.length - 1;
      (buttons[prevPos] as HTMLElement).focus();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const nextRow = row + 1;
      const nextPos = nextRow * columns + col;
      if (nextPos < buttons.length) {
        (buttons[nextPos] as HTMLElement).focus();
      } else {
        // 마지막 행을 넘어가면 같은 열의 첫 번째 행으로
        (buttons[col] as HTMLElement).focus();
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prevRow = row - 1;
      if (prevRow >= 0) {
        const prevPos = prevRow * columns + col;
        (buttons[prevPos] as HTMLElement).focus();
      } else {
        // 첫 번째 행을 넘어가면 같은 열의 마지막 행으로
        const lastRow = Math.floor((buttons.length - 1) / columns);
        const targetPos = lastRow * columns + col;
        if (targetPos < buttons.length) {
          (buttons[targetPos] as HTMLElement).focus();
        } else {
          // 해당 위치에 버튼이 없으면 마지막 버튼으로
          // eslint-disable-next-line unicorn/prefer-at
          (buttons[buttons.length - 1] as HTMLElement).focus();
        }
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      (target as HTMLButtonElement).click();
    }
  };
</script>

<svelte:window onkeydown={onKeydown} />

<div
  bind:this={containerElement}
  style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
  class={css({ display: 'grid', gap: '8px', padding: '8px' })}
>
  {#each items as { label, value, color } (value)}
    <button
      style:background-color={value === 'none' ? 'transparent' : color}
      style:outline-color={value === 'none' || value === 'white' ? token('colors.border.default') : color}
      class={center({
        borderWidth: '1px',
        borderRadius: shape === 'circle' ? 'full' : '4px',
        outlineWidth: currentValue === value ? '2px' : '0',
        outlineOffset: '1px',
        size: '20px',
        position: 'relative',
        _focus: {
          outlineWidth: '2px',
        },
      })}
      aria-label={label}
      data-active={currentValue === value}
      onclick={() => {
        onSelect(value);
        close();
      }}
      type="button"
    >
      {#if showNone && value === 'none'}
        <div
          class={css({
            position: 'absolute',
            inset: '0',
            margin: 'auto',
            width: '1px',
            height: '14px',
            backgroundColor: 'text.disabled',
            transform: 'rotate(45deg)',
          })}
        ></div>
      {/if}
    </button>
  {/each}
</div>
