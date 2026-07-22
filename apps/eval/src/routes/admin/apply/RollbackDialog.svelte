<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
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

<Modal
  style={css.raw({ width: 'full', maxWidth: '560px', maxHeight: '[80vh]', overflowY: 'auto', padding: '24px' })}
  onclose={onCancel}
  open={true}
>
  <h2 class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default', marginBottom: '4px' })}>
    이전 값으로 롤백할까요?
  </h2>
  <p class={css({ fontSize: '13px', color: 'text.muted', marginBottom: '16px', lineHeight: '[1.5]' })}>
    {stageLabel} 단계를 이 적용 이전에 보존해둔 값(prev)으로 되돌립니다.
  </p>

  <p class={css({ fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>system</p>
  <LineDiff entries={systemDiff} />
  <p class={css({ marginTop: '10px', fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>tools</p>
  <LineDiff entries={toolsDiff} />

  <p class={css({ marginTop: '14px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{error ?? ''}</p>

  <div class={flex({ gap: '10px', justifyContent: 'flex-end' })}>
    <Button onclick={onCancel} size="md" variant="secondary">취소</Button>
    <Button loading={pending} onclick={onConfirm} size="md" variant="danger">롤백</Button>
  </div>
</Modal>
