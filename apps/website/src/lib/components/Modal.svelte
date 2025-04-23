<script lang="ts">
  import { portal } from '$lib/actions';
  import { css } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    open: boolean;
    children: Snippet;
    style?: SystemStyleObject;
    onclose?: () => void;
  };

  let { open = $bindable(), children, style, onclose }: Props = $props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (open = false)} />

{#if open}
  <div class={css({ position: 'fixed', inset: '0', zIndex: '50' })} use:portal>
    <div
      class={css({ position: 'absolute', inset: '0', backgroundColor: 'gray.900/24' })}
      onclick={() => {
        open = false;
        onclose?.();
      }}
      onkeypress={null}
      role="button"
      tabindex="-1"
    ></div>

    <div
      class={css({
        position: 'absolute',
        inset: '0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: { base: '20px', lg: '40px' },
        width: 'full',
        margin: 'auto',
        pointerEvents: 'none',
      })}
    >
      <div
        class={css(
          {
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            flexGrow: '1',
            borderWidth: '1px',
            borderRadius: '16px',
            backgroundColor: 'white',
            padding: '20px',
            pointerEvents: 'auto',
            height: '[fit-content]',
            width: 'full',
            maxWidth: '720px',
            maxHeight: '738px',
            overflow: 'hidden',
          },
          style,
        )}
      >
        <div class={css({ height: 'full', overflowY: 'auto' })}>
          <section class={css({ display: 'contents' })}>
            {@render children()}
          </section>
        </div>
      </div>
    </div>
  </div>
{/if}
