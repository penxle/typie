<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';

  type Props = {
    title: string;
    count?: number;
    moreHref?: string;
  };

  let { title, count, moreHref }: Props = $props();
</script>

<div class={flex({ alignItems: 'center', gap: '10px', minWidth: '0', marginBottom: '16px' })}>
  <h2 class={css({ fontSize: '18px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>
    {title}
    {#if count !== undefined}
      <span
        class={css({ marginLeft: '2px', fontSize: '13px', fontWeight: 'medium', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}
      >
        {count}
      </span>
    {/if}
  </h2>
  {#if moreHref}
    <a
      class={flex({
        alignItems: 'center',
        flexShrink: '0',
        gap: '2px',
        marginLeft: 'auto',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'text.muted',
        transition: 'colors',
        _hover: { color: 'text.default' },
      })}
      href={moreHref}
    >
      더 보기
      <Icon icon={ChevronRightIcon} size={14} />
    </a>
  {/if}
</div>
