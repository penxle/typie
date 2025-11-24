<script generics="T" lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { disassemble } from 'es-hangul';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import DropdownMenu from './DropdownMenu.svelte';
  import DropdownMenuItem from './DropdownMenuItem.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    value: T;
    items: { value: T; label: string }[];
    style?: SystemStyleObject;
    label: string;
    disabled?: boolean;
    onchange: (value: T, options?: { shouldFocus?: boolean }) => void;
    onEscape?: () => void;
    getLabel?: (value: T) => string;
    renderItem?: Snippet<[{ value: T; label: string }]>;
    extraItems?: { onclick: () => void; content: Snippet }[];
  };

  let { value, items, style, label, disabled = false, onchange, onEscape, getLabel, renderItem, extraItems = [] }: Props = $props();

  let anchorElement: HTMLDivElement | undefined = $state();
  let floatingElement: HTMLDivElement | undefined = $state();

  const { anchor: anchorAction, floating: floatingAction } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    onClickOutside: (event) => {
      if (anchorElement?.contains(event.target as Node)) {
        return;
      }
      close();
    },
  });

  let opened = $state(false);
  let inputElement: HTMLInputElement | undefined = $state();
  let chevronElement: HTMLButtonElement | undefined = $state();
  let inputValue = $state('');
  let isFocused = $state(false);

  const currentLabel = $derived.by(() => {
    if (getLabel) {
      return getLabel(value);
    }
    const item = items.find((i) => i.value === value);
    return item?.label ?? '';
  });

  $effect(() => {
    if (!isFocused && !opened) {
      inputValue = currentLabel;
    }
  });

  const open = () => {
    opened = true;
  };

  const close = () => {
    opened = false;
  };

  const handleFocus = () => {
    isFocused = true;
    open();
    inputValue = '';
    tick().then(() => {
      inputElement?.select();
    });
  };

  const handleBlur = (e: FocusEvent) => {
    isFocused = false;

    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget && floatingElement?.contains(relatedTarget)) {
      return;
    }

    close();
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) return;

    if (e.key === 'Escape') {
      inputValue = currentLabel;
      inputElement?.blur();
      close();
      onEscape?.();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();

      // NOTE: 현재 선택된 항목이 필터링된 목록에 있으면 에디터로 포커스, 없으면 첫 번째 항목 선택
      const currentItemInFiltered = filteredItems.find((item) => item.value === value);
      if (currentItemInFiltered) {
        onchange(value, { shouldFocus: true });
      } else {
        const firstItem = filteredItems[0];
        if (firstItem) {
          onchange(firstItem.value, { shouldFocus: true });
        }
      }

      inputElement?.blur();
      close();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();

      if (filteredItems.length === 0) return;

      const currentIndex = filteredItems.findIndex((i) => i.value === value);

      let newIndex: number;
      if (e.key === 'ArrowDown') {
        if (currentIndex === -1 || currentIndex >= filteredItems.length - 1) {
          newIndex = 0;
        } else {
          newIndex = currentIndex + 1;
        }
      } else {
        if (currentIndex === -1 || currentIndex <= 0) {
          newIndex = filteredItems.length - 1;
        } else {
          newIndex = currentIndex - 1;
        }
      }

      const newItem = filteredItems[newIndex];
      if (newItem) {
        onchange(newItem.value);
        tick().then(() => {
          inputElement?.select();
        });
      }
    }
  };

  const filteredItems = $derived.by(() => {
    const query = disassemble(inputValue.toLowerCase().trim());
    if (!query) return items;
    return items.filter((item) => disassemble(item.label.toLowerCase()).includes(query));
  });
</script>

<div
  bind:this={anchorElement}
  class={css(
    {
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      borderRadius: '4px',
      paddingX: '4px',
      height: '24px',
      _hover: {
        backgroundColor: 'surface.muted',
      },
      '& > input:focus': {
        backgroundColor: 'surface.muted',
      },
    },
    style,
  )}
  use:anchorAction
  use:tooltip={{ message: isFocused ? null : label, delay: 200, arrow: false }}
>
  <input
    bind:this={inputElement}
    class={css({
      flexGrow: '1',
      width: 'full',
      paddingRight: '16px',
      fontSize: '14px',
      color: 'text.subtle',
      textAlign: 'left',
      backgroundColor: 'transparent',
      border: 'none',
      outline: 'none',
      textOverflow: 'ellipsis',
    })}
    {disabled}
    onblur={handleBlur}
    onfocus={handleFocus}
    onkeydown={handleKeydown}
    placeholder={currentLabel}
    type="text"
    bind:value={inputValue}
  />

  <button
    bind:this={chevronElement}
    class={css({
      pointerEvents: opened ? 'auto' : 'none',
      cursor: 'pointer',
    })}
    onclick={() => {
      inputElement?.blur();
      close();
    }}
    type="button"
  >
    <Icon
      style={css.raw({
        position: 'absolute',
        right: '4px',
        top: '1/2',
        translate: 'auto',
        translateY: '-1/2',
        color: 'text.faint',
        transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
        transitionDuration: '150ms',
      })}
      icon={ChevronDownIcon}
      size={16}
    />
  </button>
</div>

{#if opened}
  <div
    bind:this={floatingElement}
    class={css({
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderBottomRadius: '4px',
      backgroundColor: 'surface.default',
      zIndex: 'menu',
      boxShadow: 'small',
      overflow: 'hidden',
    })}
    use:floatingAction
    in:fly={{ y: -5, duration: 150 }}
  >
    <DropdownMenu
      autoFocus={false}
      onclose={() => {
        close();
        onEscape?.();
      }}
      {opened}
    >
      {#each filteredItems as item (item.value)}
        <DropdownMenuItem
          active={value === item.value}
          onclick={() => {
            onchange(item.value, { shouldFocus: true });
            close();
          }}
        >
          {#if renderItem}
            {@render renderItem(item)}
          {:else}
            {item.label}
          {/if}
        </DropdownMenuItem>
      {/each}

      {#each extraItems as extraItem, i (i)}
        <DropdownMenuItem
          onclick={() => {
            extraItem.onclick();
            close();
          }}
        >
          {@render extraItem.content()}
        </DropdownMenuItem>
      {/each}
    </DropdownMenu>
  </div>
{/if}
