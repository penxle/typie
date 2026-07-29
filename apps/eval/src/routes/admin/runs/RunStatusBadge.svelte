<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = { status: string };
  const { status }: Props = $props();

  const LABELS: Record<string, string> = { pending: '대기', running: '실행 중', done: '성공', failed: '실패', cancelled: '취소됨' };
</script>

<span
  class={css({
    display: 'inline-flex',
    alignItems: 'center',
    height: '20px',
    paddingX: '8px',
    borderRadius: 'full',
    fontSize: '11px',
    fontWeight: 'bold',
    flexShrink: '0',
    backgroundColor:
      status === 'done'
        ? 'accent.success.subtle'
        : status === 'failed'
          ? 'accent.danger.subtle'
          : status === 'running'
            ? 'accent.info.subtle'
            : 'surface.muted',
    color:
      status === 'done'
        ? 'text.success'
        : status === 'failed'
          ? 'text.danger'
          : status === 'running'
            ? 'accent.info.default'
            : 'text.faint',
  })}
>
  {LABELS[status] ?? status}
</span>
