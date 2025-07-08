<script lang="ts">
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { Component } from 'svelte';

  type Props = {
    label: string;
    icon: Component;
    shortcut?: string;
    active: boolean;
    onclick: () => void;
  };

  let { label, icon, shortcut, active, onclick }: Props = $props();
</script>

<svelte:window
  onkeydown={(e) => {
    if (shortcut && e.key.toUpperCase() === shortcut && !e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      onclick();
    }
  }}
/>

<button
  class={center({
    position: 'relative',
    borderRadius: '8px',
    size: '36px',
    color: 'text.subtle',
    transition: 'common',
    _hover: {
      color: 'text.default',
      backgroundColor: 'surface.subtle',
    },
    _pressed: {
      color: 'accent.brand.default',
      backgroundColor: 'accent.brand.subtle',
    },
  })}
  aria-pressed={active}
  {onclick}
  type="button"
  use:tooltip={{ placement: 'top', message: label, trailing: shortcut }}
>
  <Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} {icon} size={20} />
</button>
