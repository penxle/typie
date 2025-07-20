<script lang="ts">
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { center } from '$styled-system/patterns';
  import type { Component } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
  import type { SystemStyleObject } from '$styled-system/types';

  type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

  type Props = {
    as?: 'button' | 'a';
    icon: Component;
    label: string;
    active?: boolean;
    iconStyle?: SystemStyleObject;
    keys?: [...ModifierKey[], string];
  } & (HTMLButtonAttributes | HTMLAnchorAttributes);

  let { as = 'button', icon, label, active = false, iconStyle, keys, ...rest }: Props = $props();
</script>

<svelte:element
  this={as}
  class={center({
    borderRadius: '8px',
    size: '32px',
    color: 'text.faint',
    transition: 'common',
    _hover: {
      color: 'text.subtle',
      backgroundColor: 'interactive.hover',
    },
    _pressed: {
      color: 'text.subtle',
      backgroundColor: 'interactive.hover',
    },
  })}
  aria-pressed={active}
  type={as === 'button' ? 'button' : undefined}
  use:tooltip={{ message: label, placement: 'right', offset: 12, keys }}
  {...rest}
>
  <Icon style={iconStyle} {icon} size={20} />
</svelte:element>
