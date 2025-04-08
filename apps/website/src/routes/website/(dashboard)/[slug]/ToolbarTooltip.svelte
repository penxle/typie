<script lang="ts">
  import { createFloatingActions } from '$lib/actions';
  import { css } from '$styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = {
    label: string;
    children: Snippet;
  };

  let { label, children }: Props = $props();

  let show = $state(false);
  let timer = $state<NodeJS.Timeout>();

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
  });

  const onmouseenter = () => {
    if (timer) {
      clearTimeout(timer);
    }

    timer = setTimeout(() => {
      show = true;
    }, 500);
  };

  const onmouseleave = () => {
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }

    show = false;
  };

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
      }
    };
  });
</script>

<div {onmouseenter} {onmouseleave} role="presentation" use:anchor>
  {@render children()}
</div>

{#if show}
  <div
    class={css({
      borderRadius: '2px',
      paddingX: '10px',
      paddingY: '6px',
      fontSize: '12px',
      fontWeight: 'medium',
      color: 'gray.100',
      backgroundColor: 'gray.600',
      zIndex: '50',
      pointerEvents: 'none',
    })}
    role="tooltip"
    use:floating
  >
    {label}
  </div>
{/if}
