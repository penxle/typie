<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { AdminIcon } from '$lib/components/admin';

  type Props = {
    pageNumber: number;
    totalCount: number;
  };

  let { totalCount, pageNumber = $bindable() }: Props = $props();

  const totalPages = $derived(Math.ceil(totalCount / ADMIN_ITEMS_PER_PAGE));
  const startIndex = $derived((pageNumber - 1) * ADMIN_ITEMS_PER_PAGE + 1);
  const endIndex = $derived(Math.min(pageNumber * ADMIN_ITEMS_PER_PAGE, totalCount));
</script>

<div
  class={flex({
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '16px',
    borderTopWidth: '2px',
    borderColor: 'border.default',
  })}
>
  <div class={css({ fontSize: '11px', color: 'text.muted' })}>
    SHOWING {startIndex}-{endIndex} OF {totalCount}
  </div>
  <div class={flex({ gap: '12px' })}>
    <button
      class={css({
        borderWidth: '1px',
        borderColor: 'border.default',
        padding: '6px',
        backgroundColor: 'transparent',
        color: 'text.muted',
        cursor: pageNumber === 1 ? 'not-allowed' : 'pointer',
        _disabled: { opacity: '40' },
        _hover:
          pageNumber === 1
            ? {}
            : {
                backgroundColor: 'surface.hover',
              },
      })}
      disabled={pageNumber === 1}
      onclick={() => pageNumber--}
      type="button"
    >
      <AdminIcon icon={ChevronLeftIcon} size={16} />
    </button>
    <button
      class={css({
        borderWidth: '1px',
        borderColor: 'border.default',
        padding: '6px',
        backgroundColor: 'transparent',
        color: 'text.muted',
        cursor: pageNumber === totalPages ? 'not-allowed' : 'pointer',
        _disabled: { opacity: '40' },
        _hover:
          pageNumber === totalPages
            ? {}
            : {
                backgroundColor: 'surface.hover',
              },
      })}
      disabled={pageNumber === totalPages}
      onclick={() => pageNumber++}
      type="button"
    >
      <AdminIcon icon={ChevronRightIcon} size={16} />
    </button>
  </div>
</div>
