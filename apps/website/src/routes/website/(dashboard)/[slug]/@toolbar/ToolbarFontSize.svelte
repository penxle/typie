<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, Icon } from '@typie/ui/components';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import { clamp } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();

  const MIN_FONT_SIZE = 1;
  const MAX_FONT_SIZE = 200;

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
  let inputValue = $state('');
  let isFocused = $state(false);

  const currentFontSize = $derived(editor?.current.getAttributes('text_style').fontSize ?? defaultValues.fontSize);

  $effect(() => {
    if (!opened && document.activeElement !== inputElement) {
      inputValue = String(currentFontSize);
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
    inputValue = String(currentFontSize);
    inputElement?.select();
  };

  const applyFontSize = (shouldFocus = false) => {
    const parsed = Number.parseFloat(inputValue);
    if (!Number.isNaN(parsed) && parsed !== currentFontSize) {
      const clamped = clamp(parsed, MIN_FONT_SIZE, MAX_FONT_SIZE);
      const chain = editor?.current.chain().setFontSize(clamped);
      if (shouldFocus) {
        chain?.focus();
      }
      chain?.run();
    }
  };

  const handleBlur = (e: FocusEvent) => {
    isFocused = false;

    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget && floatingElement?.contains(relatedTarget)) {
      return;
    }

    close();
  };

  $effect(() => {
    if (!isFocused && inputValue) {
      applyFontSize();
    }
  });

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      applyFontSize(true);
      inputElement?.blur();
      close();
    } else if (e.key === 'Escape') {
      inputValue = String(currentFontSize);
      inputElement?.blur();
      close();
      editor?.current.commands.focus();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const current = Number.parseFloat(inputValue) || currentFontSize;
      const sortedSizes = values.fontSize.map(({ value }) => value).toSorted((a, b) => a - b);
      const currentIndex = sortedSizes.findIndex((size) => size >= current);

      let newIndex: number;
      if (e.key === 'ArrowDown') {
        if (currentIndex === -1) {
          newIndex = sortedSizes.length - 1;
        } else if (currentIndex >= sortedSizes.length - 1) {
          newIndex = 0;
        } else {
          newIndex = currentIndex + 1;
        }
      } else {
        if (currentIndex === -1) {
          newIndex = 0;
        } else if (currentIndex <= 0) {
          newIndex = sortedSizes.length - 1;
        } else {
          newIndex = currentIndex - 1;
        }
      }

      const newValue = sortedSizes[newIndex];
      if (newValue !== undefined) {
        inputValue = String(newValue);
        editor?.current.chain().setFontSize(newValue).run();
        tick().then(() => {
          inputElement?.select();
        });
      }
    }
  };
</script>

<div class={css({ position: 'relative', width: '50px' })}>
  <div
    bind:this={anchorElement}
    class={css({
      display: 'flex',
      alignItems: 'center',
      borderRadius: '4px',
      paddingX: '4px',
      height: '24px',
      _hover: {
        backgroundColor: 'surface.muted',
      },
      _focusWithin: {
        backgroundColor: 'surface.muted',
      },
    })}
    use:anchorAction
    use:tooltip={{ message: isFocused ? null : '폰트 크기', delay: 200, arrow: false }}
  >
    <input
      bind:this={inputElement}
      class={css({
        flexGrow: '1',
        width: 'full',
        fontSize: '14px',
        color: 'text.subtle',
        textAlign: 'left',
        backgroundColor: 'transparent',
        border: 'none',
        outline: 'none',
      })}
      disabled={!editor?.current.can().setFontSize(defaultValues.fontSize)}
      onblur={handleBlur}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
      placeholder={String(currentFontSize)}
      type="text"
      bind:value={inputValue}
    />

    <Icon
      style={css.raw({
        position: 'absolute',
        right: '4px',
        top: '1/2',
        translate: 'auto',
        translateY: '-1/2',
        color: 'text.faint',
        pointerEvents: 'none',
        transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
        transitionDuration: '150ms',
      })}
      icon={ChevronDownIcon}
      size={16}
    />
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
          editor?.current.commands.focus();
        }}
        {opened}
      >
        {#each values.fontSize as { label, value } (value)}
          <DropdownMenuItem
            active={currentFontSize === value}
            onclick={() => {
              editor?.current.chain().focus().setFontSize(value).run();
              close();
            }}
          >
            {label}
          </DropdownMenuItem>
        {/each}
      </DropdownMenu>
    </div>
  {/if}
</div>
