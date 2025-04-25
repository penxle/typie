<script lang="ts">
  import { scale } from 'svelte/transition';
  import { css } from '$styled-system/css';
  import type { ArrowAction, FloatingAction } from './floating.svelte';

  type Props = {
    message: string;
    floating: FloatingAction;
    arrow: ArrowAction;
  };

  let { message, floating, arrow }: Props = $props();
</script>

<div
  class={css({
    borderRadius: '4px',
    paddingX: '8px',
    paddingY: '4px',
    fontSize: '12px',
    fontWeight: 'semibold',
    color: 'white',
    backgroundColor: 'gray.700',
    boxShadow: 'medium',
    zIndex: '50',
    pointerEvents: 'none',
  })}
  role="tooltip"
  use:floating
  transition:scale={{ start: 0.9, duration: 200 }}
>
  <span>{message}</span>

  <div
    class={css({
      borderTopLeftRadius: '2px',
      size: '8px',
      backgroundColor: 'gray.700',
    })}
    use:arrow
  ></div>
</div>
