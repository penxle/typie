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

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
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
      },
      style,
    )}
    aria-pressed={active}
    {onclick}
    type="button"
  >
    <ToolbarIcon {icon} />
    <span class={css({ fontSize: '11px' })}>{label}</span>
  </button>
{:else if size === 'small'}
  <ToolbarTooltip {label}>
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
        },
        style,
      )}
      aria-pressed={active}
      {onclick}
      type="button"
    >
      <ToolbarIcon {icon} />
    </button>
  </ToolbarTooltip>
{/if}
