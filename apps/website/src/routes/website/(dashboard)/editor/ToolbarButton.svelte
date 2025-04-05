<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarTooltip from './ToolbarTooltip.svelte';
  import type { Component } from 'svelte';

  type Props = {
    size: 'large' | 'small';
    icon: Component;
    label: string;
    active?: boolean;
    onclick?: () => void;
  };

  let { size, icon, label, active = false, onclick }: Props = $props();
</script>

{#if size === 'large'}
  <button
    class={center({
      flexDirection: 'column',
      gap: '4px',
      borderRadius: '4px',
      size: '54px',
      _hover: { backgroundColor: 'gray.100' },
      _pressed: { backgroundColor: 'gray.200' },
    })}
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
      class={center({
        flexDirection: 'column',
        gap: '4px',
        borderRadius: '4px',
        size: '24px',
        _hover: { backgroundColor: 'gray.100' },
        _pressed: { backgroundColor: 'gray.200' },
      })}
      aria-pressed={active}
      {onclick}
      type="button"
    >
      <ToolbarIcon {icon} />
    </button>
  </ToolbarTooltip>
{/if}
