<script lang="ts">
  import { css } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { HTMLDialogAttributes } from 'svelte/elements';

  type Props = {
    open: boolean;
    children: Snippet;
  } & HTMLDialogAttributes;

  let { open = $bindable(), children, ...rest }: Props = $props();

  let dialogEl: HTMLDialogElement;
  let showModal = (dialog: HTMLDialogElement) => dialog.showModal();

  $effect(() => {
    if (dialogEl)
      if (open) {
        dialogEl.showModal();
        // document.body.style.overflow = 'hidden';
      } else {
        dialogEl.close();
        // document.body.style.overflow = '';
      }
  });
</script>

<dialog
  bind:this={dialogEl}
  class={css({
    width: 'full',
    height: 'full',
    maxWidth: '[unset]',
    maxHeight: '[unset]',
    '& ::backdrop': {
      display: 'none',
    },
  })}
  onclick={(e) => {
    if (e.target === dialogEl) {
      dialogEl.close();
      open = false;
    }
  }}
  use:showModal
  {...rest}
  onsubmit={(e) => {
    e.preventDefault();
    dialogEl.close();
    open = false;
    rest.onsubmit?.(e);
  }}
>
  <div
    class={css({ position: 'absolute', inset: '0', backgroundColor: 'gray.900/24' })}
    onclick={() => {
      open = false;
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
      padding: { base: '20px', md: '40px' },
      width: 'full',
      margin: 'auto',
      pointerEvents: 'none',
    })}
  >
    <div
      class={css({
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
        flexGrow: '1',
        borderWidth: '1px',
        borderRadius: '[20px]',
        backgroundColor: 'white',
        padding: '20px',
        pointerEvents: 'auto',
        height: '[fit-content]',
        width: 'full',
        maxWidth: '720px',
        maxHeight: '738px',
        overflow: 'hidden',
      })}
    >
      <div class={css({ height: 'full', overflowY: 'auto' })}>
        <section class={css({ display: 'contents' })}>
          {@render children()}
        </section>
      </div>
    </div>
  </div>
</dialog>
