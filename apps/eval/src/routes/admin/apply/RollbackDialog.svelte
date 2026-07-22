<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { diffLines } from '$lib/domain/line-diff.ts';
  import LineDiff from './LineDiff.svelte';
  import type { StagePrompt } from '$lib/domain/admin-types.ts';

  type Props = {
    stageLabel: string;
    current: StagePrompt;
    prev: StagePrompt;
    pending: boolean;
    error: string | null;
    onConfirm: () => void;
    onCancel: () => void;
  };
  const { stageLabel, current, prev, pending, error, onConfirm, onCancel }: Props = $props();

  const systemDiff = $derived(diffLines(current.system, prev.system));
  const toolsDiff = $derived(diffLines(JSON.stringify(current.tools, null, 2), JSON.stringify(prev.tools, null, 2)));
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
    class={css({
      width: '560px',
      maxHeight: '[80vh]',
      overflowY: 'auto',
      backgroundColor: 'surface.default',
      borderRadius: '12px',
      boxShadow: 'modal',
      padding: '24px',
    })}
    aria-modal="true"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
  >
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '4px' })}>1개 단계를 프로덕션에 적용합니다</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', marginBottom: '16px', lineHeight: '[1.5]' })}>
      {stageLabel} 단계를 이 적용 이전에 보존해둔 값(prev)으로 되돌립니다.
    </p>

    <p class={css({ fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>system</p>
    <LineDiff entries={systemDiff} />
    <p class={css({ marginTop: '10px', fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>tools</p>
    <LineDiff entries={toolsDiff} />

    <p class={css({ marginTop: '14px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{error ?? ''}</p>

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
        취소
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
        {pending ? '되돌리는 중…' : '롤백'}
      </button>
    </div>
  </div>
</div>
