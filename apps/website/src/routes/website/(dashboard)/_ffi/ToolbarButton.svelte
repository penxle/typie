<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component } from 'svelte';

  type Props = {
    label: string;
    icon?: Component;
    active?: boolean;
    disabled?: boolean;
    onclick: () => void;
    style?: SystemStyleObject;
  };

  let { label, icon, active, disabled = false, onclick, style }: Props = $props();
</script>

<button
  class={css(
    {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '28px',
      borderRadius: '4px',
      borderWidth: '1px',
      borderColor: 'transparent',
      cursor: 'pointer',
      color: 'text.subtle',
      _enabled: {
        _hover: { backgroundColor: 'surface.muted' },
      },
      _disabled: { opacity: '30', cursor: 'default' },
    },
    active === true && { backgroundColor: 'surface.muted', borderColor: 'border.subtle', color: 'text.default' },
    active === false && {},
    style,
  )}
  {disabled}
  {onclick}
  title={label}
  type="button"
>
  {#if icon}
    <Icon {icon} size={16} />
  {/if}
</button>
