<script lang="ts">
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import SearchIcon from '~icons/lucide/search';
  import { graphql } from '$graphql';
  import { AdminIcon, AdminPagination, AdminTable } from '$lib/components/admin';
  import { QueryString, QueryStringNumber } from '$lib/state';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const pageNumber = new QueryStringNumber('page', 1);

  const query = graphql(`
    query AdminUsers_Query($search: String, $offset: Int!, $limit: Int!) {
      adminUsers(search: $search, offset: $offset, limit: $limit) {
        totalCount

        users {
          id
          name
          email
          role
          state
          createdAt
          avatar {
            id
            url
          }
          singleSignOns {
            id
            provider
            email
          }
          subscription {
            id
            state
          }
          credit
          sites {
            id
          }
        }
      }
    }
  `);

  $effect(() => {
    void searchQuery.current;
    untrack(() => (pageNumber.current = 1));
  });
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>USER MANAGEMENT</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'amber.400' })}>
      TOTAL USERS: {$query.adminUsers.totalCount}
    </p>
  </div>

  <div
    class={css({
      borderWidth: '2px',
      borderColor: 'amber.500',
      backgroundColor: 'gray.900',
    })}
  >
    <div class={css({ padding: '20px', borderBottomWidth: '2px', borderColor: 'amber.500' })}>
      <div class={css({ position: 'relative', maxWidth: '320px' })}>
        <AdminIcon
          style={css.raw({
            position: 'absolute',
            left: '12px',
            top: '[50%]',
            transform: 'translateY(-50%)',
            color: 'amber.500',
          })}
          icon={SearchIcon}
          size={16}
        />
        <input
          class={css({
            width: 'full',
            paddingLeft: '36px',
            paddingRight: '12px',
            paddingY: '8px',
            borderWidth: '2px',
            borderColor: 'amber.500',
            backgroundColor: 'gray.800',
            color: 'amber.500',
            fontSize: '13px',
            outline: 'none',
            caretColor: 'amber.500',
            _placeholder: {
              color: 'amber.400',
              opacity: '[0.5]',
            },
            _focus: {
              borderColor: 'amber.400',
            },
          })}
          placeholder="SEARCH NAME OR EMAIL..."
          type="text"
          bind:value={searchQuery.current}
        />
      </div>
    </div>

    <AdminTable
      columns={[
        { key: '$user', label: '사용자', width: '40%' },
        { key: '$role', label: '역할', width: '15%' },
        { key: '$state', label: '상태', width: '15%' },
        { key: '$createdAt', label: '가입일', width: '30%' },
      ]}
      data={$query.adminUsers.users}
      dataKey="id"
    >
      {#snippet $user(user)}
        <div class={flex({ alignItems: 'center', gap: '16px' })}>
          <div
            class={css({
              borderRadius: 'full',
              size: '40px',
              backgroundColor: 'amber.500',
              overflow: 'hidden',
            })}
          >
            {#if user.avatar?.url}
              <img alt={user.name} src={user.avatar.url} />
            {/if}
          </div>
          <div>
            <a
              class={css({
                fontSize: '13px',
                color: 'amber.500',
                _hover: { textDecoration: 'underline' },
              })}
              href="/admin/users/{user.id}"
            >
              {user.name}
            </a>
            <div class={css({ fontSize: '11px', fontFamily: 'mono', color: 'amber.400' })}>
              {user.email}
            </div>
          </div>
        </div>
      {/snippet}

      {#snippet $role(user)}
        <span class={css({ fontSize: '12px', color: user.role === 'ADMIN' ? 'amber.500' : 'gray.400' })}>
          [{user.role}]
        </span>
      {/snippet}

      {#snippet $state(user)}
        <span class={css({ fontSize: '12px', color: user.state === 'ACTIVE' ? 'green.400' : 'gray.400' })}>
          [{user.state}]
        </span>
      {/snippet}

      {#snippet $createdAt(user)}
        {dayjs(user.createdAt).formatAsDateTime()}
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={$query.adminUsers.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
