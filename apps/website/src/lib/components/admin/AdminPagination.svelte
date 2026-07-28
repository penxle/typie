<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';

  type Props = {
    pageNumber: number;
    totalCount: number;
  };

  let { totalCount, pageNumber = $bindable() }: Props = $props();

  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / ADMIN_ITEMS_PER_PAGE)));
  const startIndex = $derived(totalCount === 0 ? 0 : (pageNumber - 1) * ADMIN_ITEMS_PER_PAGE + 1);
  const endIndex = $derived(Math.min(pageNumber * ADMIN_ITEMS_PER_PAGE, totalCount));
</script>

<div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
  <div class={css({ fontSize: '12px', color: 'text.faint' })}>
    {totalCount}건 중 {startIndex}–{endIndex}
  </div>
  <div class={flex({ gap: '6px' })}>
    <Button style={css.raw({ borderRadius: 'full' })} disabled={pageNumber <= 1} onclick={() => pageNumber--} size="sm" variant="secondary">
      <Icon icon={ChevronLeftIcon} size={14} />
    </Button>
    <Button
      style={css.raw({ borderRadius: 'full' })}
      disabled={pageNumber >= totalPages}
      onclick={() => pageNumber++}
      size="sm"
      variant="secondary"
    >
      <Icon icon={ChevronRightIcon} size={14} />
    </Button>
  </div>
</div>
