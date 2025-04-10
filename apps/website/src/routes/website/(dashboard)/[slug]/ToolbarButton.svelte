<script lang="ts">
  import { css } from '$styled-system/css';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarTooltip from './ToolbarTooltip.svelte';
  import type { Component } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    style?: SystemStyleObject;
    size: 'large' | 'small';
    icon: Component;
    label: string;
    active?: boolean;
    disabled?: boolean;
    onclick?: () => void;
  };

  let { style, size, icon, label, active = false, disabled = false, onclick }: Props = $props();
</script>

{#if size === 'large'}
  <button
    class={css(
      {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        flexDirection: 'column',
        gap: '2px',
        borderRadius: '6px',
        size: '40px',
        _enabled: {
          _hover: { backgroundColor: 'gray.100' },
          _pressed: { backgroundColor: 'gray.100' },
        },
        _disabled: { opacity: '50' },
      },
      style,
    )}
    aria-pressed={active}
    {disabled}
    {onclick}
    type="button"
  >
    <ToolbarIcon {icon} />
    <span class={css({ fontSize: '10px' })}>{label}</span>
  </button>
{:else if size === 'small'}
  <ToolbarTooltip {label}>
    <button
      class={css(
        {
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          borderRadius: '6px',
          size: '24px',
          backgroundColor: 'gray.100',
          _enabled: {
            _hover: { backgroundColor: 'gray.200' },
            _pressed: { backgroundColor: 'gray.200' },
          },
          _disabled: { opacity: '50' },
        },
        style,
      )}
      aria-pressed={active}
      {disabled}
      {onclick}
      type="button"
    >
      <ToolbarIcon {icon} />
    </button>
  </ToolbarTooltip>
{/if}
