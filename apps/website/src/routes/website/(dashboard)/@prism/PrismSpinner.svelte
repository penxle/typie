<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import PyramidIcon from '~icons/lucide/pyramid';

  type Props = {
    label: string;
    size?: 12 | 14 | 16 | 18 | 20 | 24;
  };

  let { label, size = 16 }: Props = $props();
</script>

<div
  class={css({ display: 'inline-flex', alignItems: 'center', perspective: '[240px]', color: 'text.faint' })}
  aria-label={label}
  role="status"
>
  <div
    class={css({
      display: 'inline-flex',
      animation: '[prism-spin 1.6s linear infinite]',
      _motionReduce: { animation: 'none' },
    })}
  >
    <Icon icon={PyramidIcon} {size} />
  </div>
</div>
