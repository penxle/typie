<script lang="ts" module>
  export type PromptSetStatus = 'draft' | 'ran';
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = { status: PromptSetStatus };
  const { status }: Props = $props();

  const LABELS: Record<PromptSetStatus, string> = { draft: '초안', ran: '실행됨' };
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
    backgroundColor: status === 'ran' ? 'accent.info.subtle' : 'surface.muted',
    color: status === 'ran' ? 'accent.info.default' : 'text.faint',
  })}
>
  {LABELS[status]}
</span>
