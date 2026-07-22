<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  type Props = { pending: boolean; error: string | null; onConfirm: () => void; onCancel: () => void };
  const { pending, error, onConfirm, onCancel }: Props = $props();
</script>

<div
  class={css({
    position: 'fixed',
    inset: '0',
    backgroundColor: 'black/50',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 'modal',
  })}
  onclick={onCancel}
  onkeydown={(e) => e.key === 'Escape' && onCancel()}
  role="presentation"
>
  <div
    class={css({ width: '380px', backgroundColor: 'surface.default', borderRadius: '12px', boxShadow: 'modal', padding: '24px' })}
    aria-modal="true"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
  >
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '8px' })}>실행을 취소할까요?</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', marginBottom: '16px', lineHeight: '[1.5]' })}>
      진행 중인 문서는 중단되고 완료된 문서의 결과는 보존됩니다. 재실행하면 완료된 문서는 건너뜁니다.
    </p>

    <p class={css({ marginBottom: '10px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{error ?? ''}</p>

    <div class={flex({ gap: '8px' })}>
      <button
        class={css({
          flex: '1',
          paddingY: '9px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '8px',
          fontSize: '13px',
          color: 'text.subtle',
          cursor: 'pointer',
          transition: '[background-color 0.15s ease]',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={onCancel}
        type="button"
      >
        되돌아가기
      </button>
      <button
        class={css({
          flex: '1',
          paddingY: '9px',
          borderRadius: '8px',
          backgroundColor: 'accent.danger.default',
          color: 'text.bright',
          fontSize: '13px',
          fontWeight: 'bold',
          cursor: 'pointer',
          transition: '[background-color 0.15s ease]',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.danger.hover' },
        })}
        disabled={pending}
        onclick={onConfirm}
        type="button"
      >
        {pending ? '취소하는 중…' : '실행 취소'}
      </button>
    </div>
  </div>
</div>
