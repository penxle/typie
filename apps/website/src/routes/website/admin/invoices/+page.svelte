<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Select } from '@typie/ui/components';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { paymentInvoiceStateLabels, paymentInvoiceStateTones } from '$lib/admin-labels';
  import {
    AdminBadge,
    AdminDataTable,
    AdminDateRange,
    adminFilledControl,
    AdminFilterBar,
    AdminPageHeader,
    AdminPagination,
  } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const stateFilter = new QueryString('state', '');
  const fromFilter = new QueryString('from', '');
  const untilFilter = new QueryString('until', '');
  const pageNumber = new QueryStringNumber('page', 1);
</script>

<AdminPageHeader description={`총 ${query.data.adminInvoices.totalCount}건`} title="인보이스" />

<AdminDataTable
  columns={[
    { key: '$user', label: '유저', width: '24%' },
    { key: '$plan', label: '플랜', width: '16%' },
    { key: '$amount', label: '금액', width: '16%' },
    { key: '$state', label: '상태', width: '16%' },
    { key: '$createdAt', label: '생성일', width: '28%' },
  ]}
  data={[...query.data.adminInvoices.invoices]}
  dataKey="id"
  emptyText="조건에 맞는 인보이스가 없습니다"
>
  {#snippet filters()}
    <AdminFilterBar>
      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 상태', value: '' },
          { label: '예정', value: 'UPCOMING' },
          { label: '결제 완료', value: 'PAID' },
          { label: '연체', value: 'OVERDUE' },
          { label: '취소됨', value: 'CANCELED' },
          { label: '면제됨', value: 'WAIVED' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={stateFilter.current}
      />

      <AdminDateRange
        bind:end={
          () => untilFilter.current,
          (value) => {
            untilFilter.current = value;
            pageNumber.current = 1;
          }
        }
        bind:start={
          () => fromFilter.current,
          (value) => {
            fromFilter.current = value;
            pageNumber.current = 1;
          }
        }
      />
    </AdminFilterBar>
  {/snippet}

  {#snippet $user(invoice)}
    <a class={css({ _hover: { textDecoration: 'underline' } })} href="/admin/users/{invoice.user.id}">
      {invoice.user.name}
    </a>
  {/snippet}

  {#snippet $plan(invoice)}
    {invoice.subscription.plan.name}
  {/snippet}

  {#snippet $amount(invoice)}
    {comma(invoice.amount)}원
  {/snippet}

  {#snippet $state(invoice)}
    <AdminBadge label={paymentInvoiceStateLabels[invoice.state]} tone={paymentInvoiceStateTones[invoice.state]} />
  {/snippet}

  {#snippet $createdAt(invoice)}
    <span class={css({ color: 'text.muted' })}>{dayjs(invoice.createdAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet footer()}
    <AdminPagination totalCount={query.data.adminInvoices.totalCount} bind:pageNumber={pageNumber.current} />
  {/snippet}
</AdminDataTable>
