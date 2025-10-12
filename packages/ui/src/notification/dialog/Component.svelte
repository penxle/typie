<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '../../components';
  import { store } from './store';
  import type { AllDialog } from './store';

  type Props = {
    dialog: AllDialog;
  };

  let { dialog }: Props = $props();

  const dismiss = () => store.update((v) => v.filter((t) => t.id !== dialog.id));
</script>

<Modal
  style={css.raw({ padding: '24px', maxWidth: '440px' })}
  onclose={() => {
    if (dialog.type === 'confirm') {
      dialog.cancelHandler?.();
    }

    dismiss();
  }}
  open={true}
>
  <div class={flex({ flexDirection: 'column', gap: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>
        {dialog.title}
      </h2>

      <div class={css({ fontSize: '14px', color: 'text.subtle', whiteSpace: 'pre-wrap', lineHeight: '[1.6]' })}>
        {dialog.message}
      </div>

      {#if dialog.children}
        {@render dialog.children()}
      {/if}
    </div>

    <div class={flex({ gap: '8px', justifyContent: 'flex-end' })}>
      {#if dialog.type === 'confirm'}
        <Button
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
    </div>
  </div>
</Modal>
