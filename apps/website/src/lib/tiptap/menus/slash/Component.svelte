<script lang="ts">
  import { HorizontalDivider, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { MenuItem } from './types';

  type Props = {
    editor: Editor;
    items: MenuItem[];
    onexecute: (item: MenuItem) => void;
    onclose: () => void;
  };

  let { editor, items, onexecute, onclose }: Props = $props();

  let selectedIdx = $state(0);
  let isOnKeyboardNavigation = $state(false);

  export const handleKeyDown = (event: KeyboardEvent) => {
    if (['ArrowDown', 'ArrowUp'].includes(event.key)) {
      event.preventDefault();
      isOnKeyboardNavigation = true;

      if (event.key === 'ArrowDown') {
        selectedIdx = (selectedIdx + 1) % items.length;
      }

      if (event.key === 'ArrowUp') {
        selectedIdx = (selectedIdx - 1 + items.length) % items.length;
      }

      selectableElems[selectedIdx]?.focus();
      return true;
    }

    isOnKeyboardNavigation = false;
    if (!editor.view.hasFocus()) {
      editor.view.focus();
    }

    if (event.key === 'Escape') {
      event.stopPropagation();
      onclose();
    }

    if (event.key === 'Enter' && items[selectedIdx]) {
      event.preventDefault();
      onexecute(items[selectedIdx]);
      return true;
    }

    return false;
  };

  let selectableElems = $state<HTMLElement[]>([]);
</script>

<div
  class={flex({
    direction: 'column',
    gap: '1px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    paddingY: '4px',
    backgroundColor: 'surface.default',
    width: '210px',
    maxHeight: '340px',
    overflowY: 'auto',
    boxShadow: 'small',
  })}
  role="menu"
>
  {#each items as item, idx (item.id)}
    {#if items[idx - 1]?.group !== item.group}
      {#if idx !== 0}
        <HorizontalDivider style={css.raw({ marginY: '2px' })} color="secondary" />
      {/if}
    {/if}

    <div
      bind:this={selectableElems[idx]}
      class={flex({
        align: 'center',
        gap: '8px',
        marginX: '4px',
        borderRadius: '6px',
        padding: '4px',
        backgroundColor: selectedIdx === idx ? 'surface.muted' : undefined,
      })}
      onclick={() => onexecute(item)}
      onkeydown={handleKeyDown}
      onpointermove={() => {
        if (isOnKeyboardNavigation) {
          isOnKeyboardNavigation = false;
        } else {
          selectedIdx = idx;
        }
      }}
      role="menuitem"
      tabindex="-1"
    >
      <div class={css({ padding: '4px' })}>
        <Icon icon={item.icon} />
      </div>

      <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>{item.name}</div>
    </div>
  {:else}
    <div class={css({ paddingX: '8px', color: 'text.disabled', fontSize: '14px', fontWeight: 'semibold' })}>결과 없음</div>
  {/each}
</div>
