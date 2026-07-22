<script lang="ts" module>
  export type DerivedVariantStatus = 'draft' | 'ran' | 'adopted' | 'applied';
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = { status: DerivedVariantStatus };
  const { status }: Props = $props();

  const LABELS: Record<DerivedVariantStatus, string> = { draft: '초안', ran: '실행됨', adopted: '채택됨', applied: '적용됨' };
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
      status === 'applied'
        ? 'accent.success.subtle'
        : status === 'adopted'
          ? 'accent.warning.subtle'
          : status === 'ran'
            ? 'accent.info.subtle'
            : 'surface.muted',
    color:
      status === 'applied'
        ? 'text.success'
        : status === 'adopted'
          ? 'accent.warning.default'
          : status === 'ran'
            ? 'accent.info.default'
            : 'text.faint',
  })}
>
  {LABELS[status]}
</span>
