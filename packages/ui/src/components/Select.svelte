<script generics="T" lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import MinusIcon from '~icons/lucide/minus';
  import { Icon, Menu, MenuItem } from '../components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    value: T;
    values?: T[];
    items: {
      icon?: Component;
      label: string;
      description?: string;
      value: T;
    }[];
    onselect?: (value: T) => void;
    chevron?: boolean;
  };

  let { style, value = $bindable(), values, items = [], onselect, chevron = true }: Props = $props();

  const selectedValues = $derived(values ?? [value]);
  const isIndeterminate = $derived(selectedValues.some((v) => selectedValues[0] !== v));

  const displayItem = $derived(
    (() => {
      if (isIndeterminate) {
        const selectedItems = items.filter((item) => selectedValues.includes(item.value));
        return {
          label: selectedItems.map((item) => item.label).join(', '),
          icon: MinusIcon,
        };
      }
      return items.find((item) => item.value === selectedValues[0]);
    })(),
  );
</script>

<Menu disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]', maxWidth: '240px' })} offset={4} placement="bottom-end">
  {#snippet button({ open }: { open: boolean })}
    <button
      class={cx(
        'group',
        css(
          {
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            borderRadius: '6px',
            paddingX: '8px',
            paddingY: '4px',
            transition: 'common',
            _hover: { backgroundColor: 'surface.muted' },
            _expanded: { backgroundColor: 'surface.muted' },
          },
          style,
        ),
      )}
      aria-expanded={open}
      type="button"
    >
      {#if displayItem}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          {#if displayItem.icon}
            <Icon style={css.raw({ color: 'text.faint' })} icon={displayItem.icon} size={14} />
          {/if}

          <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
            {displayItem.label}
          </span>
        </div>

        {#if chevron}
          <Icon
            style={css.raw({ color: 'text.faint', '& *': { strokeWidth: '[1.5px]' } })}
            icon={open ? ChevronUpIcon : ChevronDownIcon}
            size={14}
          />
        {/if}
      {/if}
    </button>
  {/snippet}

  {#each items as item (item.value)}
    <MenuItem
      onclick={() => {
        value = item.value;
        onselect?.(item.value);
      }}
    >
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '20px', flexGrow: '1' })}>
        <div class={flex({ alignItems: 'flex-start', gap: '8px' })}>
          {#if item.icon}
            <div class={center({ height: '[1lh]' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={item.icon} size={14} />
            </div>
          {/if}

          <div class={flex({ flexDirection: 'column', gap: '4px' })}>
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {item.label}
            </span>

            {#if item.description}
              <span class={css({ fontSize: '11px', color: 'text.faint', wordBreak: 'keep-all' })}>{item.description}</span>
            {/if}
          </div>
        </div>

        {#if selectedValues.includes(item.value)}
          {#if isIndeterminate}
            <Icon style={css.raw({ color: 'text.subtle' })} icon={MinusIcon} size={14} />
          {:else}
            <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
          {/if}
        {:else}
          <div style:width="14px"></div>
        {/if}
      </div>
    </MenuItem>
  {/each}
</Menu>
