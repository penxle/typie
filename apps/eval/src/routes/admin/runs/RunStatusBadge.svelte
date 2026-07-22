<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { RunStatus } from '$lib/domain/admin-types.ts';

  type Props = { status: RunStatus };
  const { status }: Props = $props();

  const LABELS: Record<RunStatus, string> = { running: '실행 중', succeeded: '성공', failed: '실패', cancelled: '취소됨' };
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
      status === 'succeeded'
        ? 'accent.success.subtle'
        : status === 'failed'
          ? 'accent.danger.subtle'
          : status === 'running'
            ? 'accent.info.subtle'
            : 'surface.muted',
    color:
      status === 'succeeded'
        ? 'text.success'
        : status === 'failed'
          ? 'text.danger'
          : status === 'running'
            ? 'accent.info.default'
            : 'text.faint',
  })}
>
  {LABELS[status]}
</span>
