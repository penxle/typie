<script lang="ts">
  import { sineOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import { portal } from '$lib/actions';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    open: boolean;
    children: Snippet;
    style?: SystemStyleObject;
    onclose?: () => void;
  };

  let { open = $bindable(), children, style, onclose }: Props = $props();

  const close = () => {
    open = false;
    onclose?.();
  };
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (open = false)} />

{#if open}
  <div class={center({ position: 'fixed', inset: '0', zIndex: '50' })} use:portal>
    <div
      class={css({
        position: 'fixed',
        inset: '0',
        backgroundColor: 'black/25',
        backdropFilter: 'auto',
        backdropBlur: '4px',
      })}
      onclick={close}
      role="none"
      transition:fade|global={{ duration: 150, easing: sineOut }}
    ></div>

    <div
      class={css(
        {
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          borderWidth: '1px',
          borderRadius: '12px',
          width: 'full',
          maxWidth: '720px',
          height: 'fit',
          backgroundColor: 'white',
          boxShadow: 'large',
          overflowY: 'auto',
        },
        style,
      )}
      transition:fly|global={{ y: 5, duration: 150, easing: sineOut }}
    >
      {@render children()}
    </div>
  </div>
{/if}
