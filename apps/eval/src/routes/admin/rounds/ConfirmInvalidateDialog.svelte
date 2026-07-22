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
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '8px' })}>라운드를 무효화할까요?</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', marginBottom: '16px', lineHeight: '[1.5]' })}>
      이 라운드의 태스크가 모두 삭제됩니다. 아직 판정이 없는 라운드만 무효화할 수 있습니다.
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
        {pending ? '무효화하는 중…' : '무효화'}
      </button>
    </div>
  </div>
</div>
