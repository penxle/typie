<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';

  type Props = {
    count: number;
    href: string;
  };

  let { count, href }: Props = $props();
</script>

<div class={flex({ marginTop: '-8px', paddingBottom: '16px', pointerEvents: 'none' })}>
  <a
    class={flex({
      alignItems: 'center',
      gap: '4px',
      fontSize: '13px',
      fontWeight: 'medium',
      color: 'text.muted',
      transition: 'colors',
      pointerEvents: 'auto',
      _hover: { color: 'text.default' },
    })}
    {href}
  >
    <span>이 스페이스의 글 {count}개 더</span>
    <Icon icon={ChevronRightIcon} size={14} />
  </a>
</div>
