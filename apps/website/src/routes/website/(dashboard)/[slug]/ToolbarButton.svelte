<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Component } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    size: 'large' | 'small';
    icon: Component;
    label: string;
    keys?: TooltipParameter['keys'];
    active?: boolean;
    disabled?: boolean;
    onclick?: () => void;
  };

  let { style, size, icon, label, keys, active = false, disabled = false, onclick }: Props = $props();
</script>

{#if size === 'large'}
  <button
    class={css(
      {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        flexDirection: 'column',
        gap: '4px',
        borderRadius: '4px',
        size: '48px',
        color: 'text.subtle',
        transition: 'common',
        _enabled: {
          _hover: { color: 'text.brand' },
          _pressed: { color: 'text.brand' },
        },
        _disabled: { opacity: '50' },
        flexShrink: '0',
      },
      style,
    )}
    aria-pressed={active}
    {disabled}
    {onclick}
    type="button"
  >
    <ToolbarIcon {icon} />
    <span class={css({ fontSize: '11px' })}>{label}</span>
  </button>
{:else if size === 'small'}
  <button
    class={css(
      {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        borderRadius: '4px',
        size: '24px',
        color: 'text.subtle',
        _enabled: {
          _hover: { color: 'text.brand' },
          _pressed: { color: 'text.brand' },
        },
        _disabled: { opacity: '50' },
        flexShrink: '0',
      },
      style,
    )}
    aria-pressed={active}
    {disabled}
    {onclick}
    type="button"
    use:tooltip={{ message: label, keys, delay: 1000, arrow: false }}
  >
    <ToolbarIcon {icon} />
  </button>
{/if}
