<script generics="T" lang="ts">
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Component } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    value: T;
    items: {
      icon?: Component;
      label: string;
      description?: string;
      value: T;
    }[];
    onselect?: (value: T) => void;
    chevron?: boolean;
  };

  let { style, value = $bindable(), items = [], onselect, chevron = true }: Props = $props();

  const item = $derived(items.find((item) => item.value === value));
</script>

<Menu disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]', maxWidth: '240px' })} offset={4} placement="bottom-end">
  {#snippet button({ open })}
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
      {#if item}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          {#if item.icon}
            <Icon style={css.raw({ color: 'text.faint' })} icon={item.icon} size={14} />
          {/if}

          <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
            {item.label}
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

        <Icon
          style={css.raw({
            color: 'text.subtle',
            visibility: item.value === value ? 'visible' : 'hidden',
          })}
          icon={CheckIcon}
          size={14}
        />
      </div>
    </MenuItem>
  {/each}
</Menu>
