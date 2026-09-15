<script generics="T" lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { Icon, Menu, MenuItem } from '../components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    value: T;
    items: {
      icon?: Component;
      label: string;
      description?: string;
      trailing?: Component;
      value: T;
    }[];
    disabled?: boolean;
    // eslint-disable-next-line @typescript-eslint/no-invalid-void-type
    onselect?: (value: T) => void | Promise<void> | boolean | Promise<boolean>;
    chevron?: boolean;
  };

  let { style, value = $bindable(), items = [], disabled, onselect, chevron = true }: Props = $props();

  const displayItem = $derived(items.find((item) => item.value === value));
</script>

<Menu disableAutoUpdate {disabled} listStyle={css.raw({ minWidth: '[initial]', maxWidth: '280px' })} offset={4} placement="bottom-end">
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
            _hover: { backgroundColor: 'surface.hover' },
            _expanded: { backgroundColor: 'surface.active' },
            _disabled: {
              opacity: '40',
            },
          },
          style,
        ),
      )}
      aria-disabled={disabled}
      aria-expanded={open}
      aria-haspopup="listbox"
      {disabled}
      onclick={() => {
        if (disabled) {
          return;
        }
      }}
      tabindex={disabled ? -1 : 0}
      type="button"
    >
      {#if displayItem}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          {#if displayItem.icon}
            <Icon style={css.raw({ color: 'text.muted' })} icon={displayItem.icon} size={14} />
          {/if}

          <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>
            {displayItem.label}
          </span>
        </div>

        {#if chevron}
          <Icon
            style={css.raw({ color: 'text.muted', '& *': { strokeWidth: '[1.5px]' } })}
            icon={open ? ChevronUpIcon : ChevronDownIcon}
            size={14}
          />
        {/if}
      {/if}
    </button>
  {/snippet}

  {#each items as item (item.value)}
    <MenuItem
      onclick={async () => {
        const ret = await onselect?.(item.value);
        if (ret === false) {
          return;
        }
        value = item.value;
      }}
    >
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '20px', flexGrow: '1' })}>
        <div class={flex({ alignItems: 'flex-start', gap: '8px' })}>
          {#if item.icon}
            <div class={center({ height: '[1lh]' })}>
              <Icon style={css.raw({ color: 'text.muted' })} icon={item.icon} size={14} />
            </div>
          {/if}

          <div class={flex({ flexDirection: 'column', gap: '4px' })}>
            <span class={flex({ alignItems: 'center', gap: '6px', fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>
              {item.label}
              {#if item.trailing}
                <item.trailing />
              {/if}
            </span>

            {#if item.description}
              <span class={css({ fontSize: '11px', color: 'text.muted', wordBreak: 'keep-all' })}>{item.description}</span>
            {/if}
          </div>
        </div>

        {#if item.value === value}
          <Icon style={css.raw({ color: 'accent.default' })} icon={CheckIcon} size={14} />
        {:else}
          <div style:width="14px"></div>
        {/if}
      </div>
    </MenuItem>
  {/each}
</Menu>
