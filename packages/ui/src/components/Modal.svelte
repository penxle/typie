<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { cubicOut } from 'svelte/easing';
  import { fade, scale } from 'svelte/transition';
  import { focusTrap, portal } from '../actions';
  import { pushEscapeHandler } from '../utils';
  import RingSpinner from './RingSpinner.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Options as FocusTrapOptions } from 'focus-trap';
  import type { Snippet } from 'svelte';

  type Props = {
    open: boolean;
    loading?: boolean;
    children: Snippet;
    style?: SystemStyleObject;
    onclose?: () => void;
    overlayPadding?: number;
    focusTrapOptions?: FocusTrapOptions;
    showBackdrop?: boolean;
    closable?: boolean;
  };

  let {
    open = $bindable(),
    children,
    style,
    onclose,
    loading = false,
    overlayPadding = 20,
    focusTrapOptions = {},
    showBackdrop = true,
    closable = true,
  }: Props = $props();

  const close = () => {
    open = false;
    onclose?.();
  };

  $effect(() => {
    if (open) {
      return pushEscapeHandler(() => {
        if (open && closable) {
          close();
          return true;
        }
        return false;
      });
    }
  });
</script>

{#if open}
  <div
    style:padding={`${overlayPadding}px`}
    class={center({ position: 'fixed', inset: '0', zIndex: 'modal', userSelect: 'none' })}
    use:focusTrap={{
      fallbackFocus: '[role="none"]',
      escapeDeactivates: false,
      returnFocusOnDeactivate: true,
      allowOutsideClick: true, // NOTE: downloadFromBase64 등 외부 클릭 허용
      ...focusTrapOptions,
    }}
    use:portal
  >
    <div
      class={css(
        {
          position: 'fixed',
          inset: '0',
          transition: 'common',
        },
        showBackdrop && {
          backgroundColor: 'black/25',
          opacity: '95',
        },
      )}
      onclick={closable ? close : undefined}
      role="none"
      in:fade|global={{ duration: 400, easing: cubicOut }}
      out:fade|global={{ duration: 280, easing: cubicOut }}
    ></div>

    {#if loading}
      <RingSpinner style={css.raw({ position: 'absolute', size: '40px', color: 'text.faint' })} />
    {:else}
      <div
        class={css(
          {
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            borderRadius: '8px',
            width: 'full',
            maxWidth: '720px',
            height: 'fit',
            maxHeight: 'full',
            backgroundColor: 'surface.default',
            boxShadow: 'modal',
            overflowY: 'auto',
            userSelect: 'text',
          },
          style,
        )}
        aria-modal="true"
        role="dialog"
        tabindex="-1"
        in:scale|global={{ start: 0.98, duration: 280, easing: cubicOut }}
        out:scale|global={{ start: 0.98, duration: 150, easing: cubicOut }}
      >
        {@render children()}
      </div>
    {/if}
  </div>
{/if}
