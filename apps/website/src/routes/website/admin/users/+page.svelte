<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select, TextInput } from '@typie/ui/components';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import SearchIcon from '~icons/lucide/search';
  import { subscriptionStateTones, userRoleLabels, userRoleTones, userStateLabels, userStateTones } from '$lib/admin-labels';
  import { AdminBadge, AdminDataTable, adminFilledControl, AdminFilterBar, AdminPageHeader, AdminPagination } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const stateFilter = new QueryString('state', '');
  const roleFilter = new QueryString('role', '');
  const pageNumber = new QueryStringNumber('page', 1);
</script>

<AdminPageHeader description={`총 ${query.data.adminUsers.totalCount}명`} title="유저" />

<AdminDataTable
  columns={[
    { key: '$user', label: '유저', width: '32%' },
    { key: '$state', label: '상태', width: '12%' },
    { key: '$role', label: '역할', width: '12%' },
    { key: '$subscription', label: '구독', width: '20%' },
    { key: '$createdAt', label: '가입일', width: '24%' },
  ]}
  data={[...query.data.adminUsers.users]}
  dataKey="id"
  emptyText="조건에 맞는 유저가 없습니다"
>
  {#snippet filters()}
    <AdminFilterBar>
      <TextInput
        style={css.raw(adminFilledControl, { maxWidth: '320px' })}
        leftIcon={SearchIcon}
        placeholder="이름, 이메일 또는 ID"
        size="sm"
        bind:value={
          () => searchQuery.current,
          (value) => {
            searchQuery.current = value;
            pageNumber.current = 1;
          }
        }
      />

      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 상태', value: '' },
          { label: '활성', value: 'ACTIVE' },
          { label: '비활성', value: 'DEACTIVATED' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={stateFilter.current}
      />

      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 역할', value: '' },
          { label: '일반', value: 'USER' },
          { label: '어드민', value: 'ADMIN' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={roleFilter.current}
      />
    </AdminFilterBar>
  {/snippet}

  {#snippet $user(user)}
    <div class={flex({ alignItems: 'center', gap: '10px' })}>
      <div class={css({ borderRadius: 'full', size: '28px', backgroundColor: 'surface.muted', overflow: 'hidden', flexShrink: '0' })}>
        {#if user.avatar?.url}
          <img alt={user.name} src={user.avatar.url} />
        {/if}
      </div>
      <div class={css({ minWidth: '0' })}>
        <a class={css({ fontWeight: 'medium', _hover: { textDecoration: 'underline' } })} href="/admin/users/{user.id}">
          {user.name}
        </a>
        <div class={css({ fontSize: '12px', color: 'text.faint', truncate: true })}>{user.email}</div>
      </div>
    </div>
  {/snippet}

  {#snippet $state(user)}
    <AdminBadge label={userStateLabels[user.state]} tone={userStateTones[user.state]} />
  {/snippet}

  {#snippet $role(user)}
    {#if user.role === 'ADMIN'}
      <AdminBadge label={userRoleLabels[user.role]} tone={userRoleTones[user.role]} />
    {:else}
      <span class={css({ color: 'text.faint' })}>{userRoleLabels[user.role]}</span>
    {/if}
  {/snippet}

  {#snippet $subscription(user)}
    {#if user.subscription}
      <AdminBadge
        label={user.subscription.plan.name}
        tone={user.subscription.state === 'ACTIVE' ? 'brand' : subscriptionStateTones[user.subscription.state]}
      />
    {:else}
      <span class={css({ color: 'text.disabled' })}>없음</span>
    {/if}
  {/snippet}

  {#snippet $createdAt(user)}
    <span class={css({ color: 'text.muted' })}>{dayjs(user.createdAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet footer()}
    <AdminPagination totalCount={query.data.adminUsers.totalCount} bind:pageNumber={pageNumber.current} />
  {/snippet}
</AdminDataTable>
