<script lang="ts">
  import { Button, Modal } from '$lib/components';
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

<Modal
  style={css.raw({ gap: '36px', maxWidth: '400px' })}
  onclose={() => {
    if (dialog.type === 'confirm') {
      dialog.cancelHandler?.();
    }

    dismiss();
  }}
  open={true}
>
  <div class={flex({ flexDirection: 'column', gap: '12px', paddingTop: '24px', paddingX: '24px' })}>
    <div class={css({ fontSize: '18px', fontWeight: 'semibold' })}>
      {dialog.title}
    </div>

    <div class={css({ fontSize: '15px', color: 'text.subtle', whiteSpace: 'pre-wrap' })}>
      {dialog.message}
    </div>

    {#if dialog.children}
      {@render dialog.children()}
    {/if}
  </div>

  <div
    class={flex({
      flexDirection: 'row-reverse',
      justifyContent: 'space-between',
      gap: '8px',
      borderTopWidth: '1px',
      padding: '16px',
      backgroundColor: 'surface.muted',
    })}
  >
    <Button
      onclick={() => {
        dialog.actionHandler?.();
        dismiss();
      }}
      size="md"
      tabindex={0}
      variant={dialog.action ?? 'primary'}
    >
      {dialog.actionLabel ?? '확인'}
    </Button>

    {#if dialog.type === 'confirm'}
      <Button
        style={css.raw({ borderColor: 'border.default' })}
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
</Modal>
