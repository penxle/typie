<script lang="ts">
  import dayjs from 'dayjs';
  import SearchIcon from '~icons/lucide/search';
  import { graphql } from '$graphql';
  import { AdminIcon, AdminPagination, AdminTable } from '$lib/components/admin';
  import { QueryString, QueryStringNumber } from '$lib/state';
  import { comma } from '$lib/utils';
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
            plan {
              id
              name
            }
          }
          credit
          sites {
            id
          }
          postCount
          totalCharacterCount
          marketingConsent
          personalIdentity {
            id
          }
          billingKey {
            id
          }
        }
      }
    }
  `);
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
      <div class={css({ position: 'relative', maxWidth: '480px' })}>
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
          placeholder="SEARCH ID, NAME OR EMAIL..."
          type="text"
          bind:value={
            () => searchQuery.current,
            (value) => {
              searchQuery.current = value;
              pageNumber.current = 1;
            }
          }
        />
      </div>
    </div>

    <AdminTable
      columns={[
        { key: '$user', label: 'USER', width: '25%' },
        { key: '$id', label: 'ID', width: '15%' },
        { key: '$subscription', label: 'SUBSCRIPTION', width: '15%' },
        { key: '$activity', label: 'ACTIVITY', width: '15%' },
        { key: '$state', label: 'STATE', width: '10%' },
        { key: '$createdAt', label: 'JOINED', width: '20%' },
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
            <div class={css({ fontSize: '11px', color: 'amber.400' })}>
              {user.email}
            </div>
          </div>
        </div>
      {/snippet}

      {#snippet $id(user)}
        <span class={css({ fontSize: '12px', color: 'gray.400' })}>
          {user.id}
        </span>
      {/snippet}

      {#snippet $subscription(user)}
        {#if user.subscription}
          <span
            class={css({
              fontSize: '12px',
              color: user.subscription.state === 'ACTIVE' ? 'green.400' : 'amber.400',
            })}
          >
            {user.subscription.plan.name}
          </span>
        {:else}
          <span class={css({ fontSize: '12px', color: 'gray.400' })}>FREE</span>
        {/if}
      {/snippet}

      {#snippet $activity(user)}
        <div class={css({ fontSize: '12px' })}>
          <div class={css({ color: 'amber.500' })}>
            {user.postCount} POSTS
          </div>
          <div class={css({ fontSize: '11px', color: 'amber.400' })}>
            {comma(user.totalCharacterCount)} CHARS
          </div>
        </div>
      {/snippet}

      {#snippet $state(user)}
        <span
          class={css({
            fontSize: '12px',
            color: user.state === 'ACTIVE' ? 'green.400' : 'red.400',
          })}
        >
          {user.state}
        </span>
      {/snippet}

      {#snippet $createdAt(user)}
        <span class={css({ fontSize: '12px', color: 'amber.400' })}>
          {dayjs(user.createdAt).formatAsDateTime()}
        </span>
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={$query.adminUsers.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
