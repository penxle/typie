<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Component } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    icon: Component;
    label: string;
    keys?: TooltipParameter['keys'];
    active?: boolean;
    disabled?: boolean;
    onclick?: () => void;
    onpointerdown?: (e: PointerEvent) => void;
  };

  let { style, icon, label, keys, active = false, disabled = false, onclick, onpointerdown }: Props = $props();
</script>

<button
  class={css(
    {
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      borderRadius: '4px',
      size: '24px',
      color: 'text.subtle',
      transition: 'common',
      _enabled: {
        _hover: { color: 'text.default', backgroundColor: 'surface.muted' },
        _pressed: { color: 'text.brand', backgroundColor: 'surface.muted' },
      },
      _disabled: { opacity: '50' },
      flexShrink: '0',
    },
    style,
  )}
  aria-label={label}
  aria-pressed={active}
  {disabled}
  {onclick}
  {onpointerdown}
  type="button"
  use:tooltip={{
    message: label,
    keys,
    arrow: false,
  }}
>
  <ToolbarIcon {icon} />
</button>
