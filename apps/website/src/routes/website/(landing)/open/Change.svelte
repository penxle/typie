<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { calculateChange, COPY, formatChange } from './open';
  import type { MetricKind, Point } from './open';

  type Props = { data: Point[]; kind: MetricKind };

  let { data, kind }: Props = $props();

  const change = $derived(calculateChange(data, kind));
  const sign = $derived(change === 0 ? 'flat' : change > 0 ? 'up' : 'down');

  const hostClass = css({
    display: 'inline-flex',
    alignItems: 'baseline',
    gap: '8px',
    fontFamily: 'mono',
    fontVariantNumeric: 'tabular-nums',
  });
  const labelClass = css({ fontSize: '12px', color: 'text.hint' });
  const valueClass = css({
    fontSize: '13px',
    fontWeight: 'medium',
    '&[data-sign="up"]': { color: 'success.default' },
    '&[data-sign="down"]': { color: 'danger.default' },
    '&[data-sign="flat"]': { color: 'text.hint' },
  });
</script>

<span class={hostClass}>
  <span class={labelClass}>{COPY[kind]}</span>
  <span class={valueClass} data-sign={sign}>{formatChange(change)}</span>
</span>
