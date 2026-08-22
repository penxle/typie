<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    label: string;
  };

  let { label }: Props = $props();
</script>

<div
  class={css({
    size: '14px',
    flexShrink: '0',
    borderRadius: 'full',
    borderWidth: '2px',
    borderColor: 'border.default',
    borderTopColor: 'text.subtle',
    animation: 'spin 1s linear infinite',
    _motionReduce: { animation: 'none' },
  })}
  aria-label={label}
  role="status"
></div>
