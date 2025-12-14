<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, Icon } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { getEditor } from '$lib/editor/context';
  import type { Mark, MarkType } from '$lib/editor/types';

  const editor = getEditor();

  const MIN_FONT_SIZE = 1;
  const MAX_FONT_SIZE = 200;

  const fontSizes = [
    { label: '8', value: 8 },
    { label: '9', value: 9 },
    { label: '10', value: 10 },
    { label: '11', value: 11 },
    { label: '12', value: 12 },
    { label: '14', value: 14 },
    { label: '16', value: 16 },
    { label: '18', value: 18 },
    { label: '20', value: 20 },
    { label: '22', value: 22 },
    { label: '24', value: 24 },
    { label: '30', value: 30 },
    { label: '36', value: 36 },
    { label: '48', value: 48 },
    { label: '60', value: 60 },
    { label: '72', value: 72 },
    { label: '96', value: 96 },
  ];

  const defaultFontSize = 12;

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

  const activeMarks = $derived(editor.activeMarks);
  const findMark = (type: string): Mark | undefined => activeMarks.uniformMarks.find((m) => m.type === type);
  const isMixed = (type: MarkType): boolean => activeMarks.mixedMarks.includes(type);

  const currentFontSize = $derived(
    isMixed('font_size') ? undefined : ((findMark('font_size') as { size?: number })?.size ?? defaultFontSize),
  );

  $effect(() => {
    if (!opened && document.activeElement !== inputElement) {
      inputValue = String(currentFontSize ?? defaultFontSize);
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
    inputValue = String(currentFontSize ?? defaultFontSize);
    inputElement?.select();
  };

  const applyFontSize = (shouldFocus = false) => {
    const parsed = Number.parseFloat(inputValue);
    if (!Number.isNaN(parsed) && parsed !== currentFontSize) {
      const clamped = clamp(parsed, MIN_FONT_SIZE, MAX_FONT_SIZE);
      editor.dispatch({ type: 'setFontSize', size: clamped });
    }
    void shouldFocus;
  };

  const handleBlur = (e: FocusEvent) => {
    isFocused = false;

    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget && (floatingElement?.contains(relatedTarget) || chevronElement?.contains(relatedTarget))) {
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
      editor.focus();
    } else if (e.key === 'Escape') {
      inputValue = String(currentFontSize ?? defaultFontSize);
      inputElement?.blur();
      close();
      editor.focus();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const current = Number.parseFloat(inputValue) || currentFontSize || defaultFontSize;
      const sortedSizes = fontSizes.map(({ value }) => value).toSorted((a, b) => a - b);
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
        editor.dispatch({ type: 'setFontSize', size: newValue });
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
      '& > input:focus': {
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
        paddingRight: '16px',
        fontSize: '14px',
        color: 'text.subtle',
        textAlign: 'left',
        backgroundColor: 'transparent',
        border: 'none',
        outline: 'none',
      })}
      disabled={!editor.can('setFontSize')}
      onblur={handleBlur}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
      placeholder={String(currentFontSize ?? defaultFontSize)}
      type="text"
      bind:value={inputValue}
    />

    <button
      bind:this={chevronElement}
      class={css({
        pointerEvents: opened ? 'auto' : 'none',
        cursor: 'pointer',
      })}
      disabled={!editor.can('setFontSize')}
      onclick={() => {
        applyFontSize(true);
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
        }}
        {opened}
      >
        {#each fontSizes as { label, value } (value)}
          <DropdownMenuItem
            active={currentFontSize === value}
            onclick={() => {
              editor.dispatch({ type: 'setFontSize', size: value });
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
