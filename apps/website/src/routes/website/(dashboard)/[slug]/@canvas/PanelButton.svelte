<script lang="ts">
  import { tooltip } from '$lib/actions';
  import { center } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    label: string;
    active: boolean;
    onclick: () => void;
    children: Snippet;
  };

  let { label, active, onclick, children }: Props = $props();
</script>

<button
  class={center({
    position: 'relative',
    borderRadius: '8px',
    size: '36px',
    color: 'text.subtle',
    backgroundColor: 'surface.subtle',
    transition: 'common',
    _hover: {
      color: 'text.default',
      backgroundColor: 'surface.muted',
    },
    _pressed: {
      color: 'text.bright',
      backgroundColor: 'accent.brand.default',
    },
  })}
  aria-pressed={active}
  {onclick}
  type="button"
  use:tooltip={{ placement: 'top', message: label }}
>
  {@render children()}
</button>
