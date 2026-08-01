<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Select } from '@typie/ui/components';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import { subscriptionStateLabels, subscriptionStateTones } from '$lib/admin-labels';
  import { AdminBadge, AdminDataTable, adminFilledControl, AdminFilterBar, AdminPageHeader, AdminPagination } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const stateFilter = new QueryString('state', '');
  const pageNumber = new QueryStringNumber('page', 1);
</script>

<AdminPageHeader description={`총 ${query.data.adminSubscriptions.totalCount}건`} title="구독" />

<AdminDataTable
  columns={[
    { key: '$user', label: '유저', width: '28%' },
    { key: '$plan', label: '플랜', width: '16%' },
    { key: '$state', label: '상태', width: '16%' },
    { key: '$startsAt', label: '시작', width: '20%' },
    { key: '$expiresAt', label: '만료', width: '20%' },
  ]}
  data={[...query.data.adminSubscriptions.subscriptions]}
  dataKey="id"
  emptyText="조건에 맞는 구독이 없습니다"
>
  {#snippet filters()}
    <AdminFilterBar>
      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 상태', value: '' },
          { label: '활성', value: 'ACTIVE' },
          { label: '활성 예정', value: 'WILL_ACTIVATE' },
          { label: '만료 예정', value: 'WILL_EXPIRE' },
          { label: '유예 기간', value: 'IN_GRACE_PERIOD' },
          { label: '만료됨', value: 'EXPIRED' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={stateFilter.current}
      />
    </AdminFilterBar>
  {/snippet}

  {#snippet $user(subscription)}
    <a class={css({ _hover: { textDecoration: 'underline' } })} href="/admin/users/{subscription.user.id}">
      {subscription.user.name}
    </a>
  {/snippet}

  {#snippet $plan(subscription)}
    {subscription.plan.name}
  {/snippet}

  {#snippet $state(subscription)}
    <AdminBadge label={subscriptionStateLabels[subscription.state]} tone={subscriptionStateTones[subscription.state]} />
  {/snippet}

  {#snippet $startsAt(subscription)}
    <span class={css({ color: 'text.muted' })}>{dayjs(subscription.startsAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet $expiresAt(subscription)}
    <span class={css({ color: 'text.muted' })}>{dayjs(subscription.expiresAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet footer()}
    <AdminPagination totalCount={query.data.adminSubscriptions.totalCount} bind:pageNumber={pageNumber.current} />
  {/snippet}
</AdminDataTable>
