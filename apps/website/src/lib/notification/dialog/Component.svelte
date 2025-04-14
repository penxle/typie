<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import { scrollLock } from '$lib/actions';
  import { Button } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { store } from './store';
  import type { AllDialog } from './store';

  type Props = {
    dialog: AllDialog;
  };

  let { dialog }: Props = $props();

  const dismiss = () => store.update((v) => v.filter((t) => t.id !== dialog.id));
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      if (dialog.type === 'confirm') {
        dialog.cancelHandler?.();
      }

      dismiss();
    }
  }}
/>

<div
  class={css({
    position: 'absolute',
    inset: '0',
    backgroundColor: 'black/25',
    pointerEvents: 'auto',
  })}
  onclick={() => dismiss()}
  onkeypress={null}
  role="button"
  tabindex="-1"
  transition:fade={{ duration: 300, easing: cubicOut }}
></div>

<div
  class={flex({
    flexDirection: 'column',
    gap: '36px',
    borderRadius: '12px',
    width: 'full',
    maxWidth: '400px',
    backgroundColor: 'white',
    boxShadow: 'large',
    overflow: 'hidden',
    zIndex: '1',
    pointerEvents: 'auto',
  })}
  use:scrollLock
  transition:fade={{ duration: 150, easing: cubicOut }}
>
  <div class={flex({ flexDirection: 'column', gap: '12px', paddingTop: '24px', paddingX: '24px' })}>
    <div class={css({ fontSize: '18px', fontWeight: 'semibold' })}>
      {dialog.title}
    </div>

    <div class={css({ fontSize: '15px', color: 'gray.700' })}>
      {dialog.message}
    </div>
  </div>

  <div
    class={css({
      display: 'flex',
      flexDirection: 'row-reverse',
      justifyContent: 'space-between',
      gap: '8px',
      borderTopWidth: '1px',
      padding: '16px',
      backgroundColor: 'gray.100',
    })}
  >
    <Button
      onclick={() => {
        dialog.actionHandler?.();
        dismiss();
      }}
      size="md"
      variant={dialog.action ?? 'primary'}
    >
      {dialog.actionLabel ?? '확인'}
    </Button>

    {#if dialog.type === 'confirm'}
      <Button
        style={css.raw({ borderColor: 'gray.200' })}
        onclick={() => {
          dialog.cancelHandler?.();
          dismiss();
        }}
        size="md"
        variant="secondary"
      >
        {dialog.cancelLabel ?? '취소'}
      </Button>
    {/if}
  </div>
</div>
