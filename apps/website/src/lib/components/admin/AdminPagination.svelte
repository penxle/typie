<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { AdminIcon } from '$lib/components/admin';
  import { ADMIN_ITEMS_PER_PAGE } from '$lib/constants';

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
    borderColor: 'amber.500',
  })}
>
  <div class={css({ fontSize: '11px', color: 'amber.500' })}>
    SHOWING {startIndex}-{endIndex} OF {totalCount}
  </div>
  <div class={flex({ gap: '12px' })}>
    <button
      class={css({
        borderWidth: '1px',
        borderColor: 'amber.500',
        padding: '6px',
        backgroundColor: 'transparent',
        color: pageNumber === 1 ? 'gray.400' : 'amber.500',
        cursor: pageNumber === 1 ? 'not-allowed' : 'pointer',
        _hover:
          pageNumber === 1
            ? {}
            : {
                backgroundColor: 'amber.500',
                color: 'gray.900',
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
        borderColor: 'amber.500',
        padding: '6px',
        backgroundColor: 'transparent',
        color: pageNumber === totalPages ? 'gray.400' : 'amber.500',
        cursor: pageNumber === totalPages ? 'not-allowed' : 'pointer',
        _hover:
          pageNumber === totalPages
            ? {}
            : {
                backgroundColor: 'amber.500',
                color: 'gray.900',
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
