<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, Icon } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { values } from '$lib/editor/values';

  const { editor } = getEditorContext();

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

  const fontSizeAttr = $derived(editor.getAttr('font_size'));
  const fontSizeValues = $derived(fontSizeAttr?.values.filter((v): v is number => v != null) ?? []);
  const currentFontSize = $derived(fontSizeValues.length === 1 ? fontSizeValues[0] : undefined);

  $effect(() => {
    if (!opened && document.activeElement !== inputElement) {
      inputValue = currentFontSize === undefined ? '' : String(currentFontSize);
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
    inputValue = currentFontSize === undefined ? '' : String(currentFontSize);
    inputElement?.select();
  };

  const applyFontSize = (shouldFocus = false) => {
    if (!inputValue) return;

    const parsed = Number.parseFloat(inputValue);
    if (!Number.isNaN(parsed) && parsed !== currentFontSize) {
      const clamped = clamp(parsed, values.minFontSize, values.maxFontSize);
      editor.dispatch({ type: 'toggleStyle', style: { type: 'font_size', size: clamped } });
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
      inputValue = currentFontSize === undefined ? '' : String(currentFontSize);
      inputElement?.blur();
      close();
      editor.focus();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const current = Number.parseFloat(inputValue) || currentFontSize;
      if (!current) return;
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
        editor.dispatch({ type: 'toggleStyle', style: { type: 'font_size', size: newValue } });
        tick().then(() => {
          inputElement?.select();
          const menuItems = floatingElement?.querySelectorAll('button[type="button"]');
          (menuItems?.[newIndex] as HTMLElement)?.scrollIntoView({ block: 'nearest' });
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
      disabled={!editor.can('toggleStyle')}
      onblur={handleBlur}
      onfocus={handleFocus}
      onkeydown={handleKeydown}
      placeholder={currentFontSize === undefined ? '-' : String(currentFontSize)}
      type="text"
      bind:value={inputValue}
    />

    <button
      bind:this={chevronElement}
      class={css({
        pointerEvents: opened ? 'auto' : 'none',
        cursor: 'pointer',
      })}
      disabled={!editor.can('toggleStyle')}
      onclick={() => {
        applyFontSize(true);
        inputElement?.blur();
        close();
        editor.focus();
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
        {#each values.fontSize as { label, value } (value)}
          <DropdownMenuItem
            active={currentFontSize === value}
            onclick={() => {
              editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'font_size', size: value } });
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
