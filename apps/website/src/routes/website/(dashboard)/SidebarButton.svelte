<script lang="ts">
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { center } from '$styled-system/patterns';
  import type { Component } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  type Props = {
    as?: 'button' | 'a';
    icon: Component;
    label: string;
    active?: boolean;
  } & (HTMLButtonAttributes | HTMLAnchorAttributes);

  let { as = 'button', icon, label, active = false, ...rest }: Props = $props();
</script>

<svelte:element
  this={as}
  class={center({
    borderRadius: '8px',
    size: '32px',
    color: 'gray.500',
    transition: 'common',
    _hover: {
      color: 'gray.700',
      backgroundColor: 'gray.200',
    },
    _pressed: {
      color: 'gray.700',
      backgroundColor: 'gray.200',
    },
  })}
  aria-pressed={active}
  type={as === 'button' ? 'button' : undefined}
  use:tooltip={{ message: label, placement: 'right', offset: 12 }}
  {...rest}
>
  <Icon {icon} size={20} />
</svelte:element>
