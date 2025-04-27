<script lang="ts">
  import { fade } from 'svelte/transition';
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
    offset: 12,
  });

  const onclick = () => {
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }

    show = false;
  };

  const onmouseenter = () => {
    if (timer) {
      clearTimeout(timer);
    }

    timer = setTimeout(() => {
      show = true;
    }, 1000);
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

<div {onclick} {onmouseenter} {onmouseleave} role="none" use:anchor>
  {@render children()}
</div>

{#if show}
  <div
    class={css({
      borderRadius: '4px',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '12px',
      fontWeight: 'medium',
      color: 'white',
      backgroundColor: 'gray.700',
      boxShadow: 'medium',
      zIndex: '50',
      pointerEvents: 'none',
    })}
    role="tooltip"
    use:floating
    in:fade|global={{ duration: 150 }}
  >
    {label}
  </div>
{/if}
