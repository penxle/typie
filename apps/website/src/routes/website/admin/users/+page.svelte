<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import SearchIcon from '~icons/lucide/search';
  import { AdminIcon, AdminPagination, AdminTable } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const pageNumber = new QueryStringNumber('page', 1);
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'text.default' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'text.default' })}>USER MANAGEMENT</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'text.muted' })}>
      TOTAL USERS: {query.data.adminUsers.totalCount}
    </p>
  </div>

  <div
    class={css({
      borderWidth: '2px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={css({ padding: '20px', borderBottomWidth: '2px', borderColor: 'border.default' })}>
      <div class={css({ position: 'relative', maxWidth: '480px' })}>
        <AdminIcon
          style={css.raw({
            position: 'absolute',
            left: '12px',
            top: '[50%]',
            transform: 'translateY(-50%)',
            color: 'text.default',
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
            borderColor: 'border.default',
            backgroundColor: 'surface.inset',
            color: 'text.default',
            fontSize: '13px',
            outline: 'none',
            caretColor: 'text.default',
            _placeholder: {
              color: 'text.hint',
            },
            _focus: {
              borderColor: 'accent.default',
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
      data={[...query.data.adminUsers.users]}
      dataKey="id"
    >
      {#snippet $user(user)}
        <div class={flex({ alignItems: 'center', gap: '16px' })}>
          <div
            class={css({
              borderRadius: 'full',
              size: '40px',
              backgroundColor: 'accent.subtle',
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
                color: 'text.default',
                _hover: { textDecoration: 'underline' },
              })}
              href="/admin/users/{user.id}"
            >
              {user.name}
            </a>
            <div class={css({ fontSize: '11px', color: 'text.muted' })}>
              {user.email}
            </div>
          </div>
        </div>
      {/snippet}

      {#snippet $id(user)}
        <span class={css({ fontSize: '12px', color: 'text.hint' })}>
          {user.id}
        </span>
      {/snippet}

      {#snippet $subscription(user)}
        {#if user.subscription}
          <span
            class={css({
              fontSize: '12px',
              color: user.subscription.state === 'ACTIVE' ? 'success.default' : 'warning.default',
            })}
          >
            {user.subscription.plan.name}
          </span>
        {:else}
          <span class={css({ fontSize: '12px', color: 'text.hint' })}>FREE</span>
        {/if}
      {/snippet}

      {#snippet $activity(user)}
        <div class={css({ fontSize: '12px' })}>
          <div class={css({ color: 'text.default' })}>
            {user.documentCount} DOCUMENTS
          </div>
          <div class={css({ fontSize: '11px', color: 'text.muted' })}>
            {comma(user.usage.totalCharacterCount)} CHARS
          </div>
        </div>
      {/snippet}

      {#snippet $state(user)}
        <span
          class={css({
            fontSize: '12px',
            color: user.state === 'ACTIVE' ? 'success.default' : 'danger.default',
          })}
        >
          {user.state}
        </span>
      {/snippet}

      {#snippet $createdAt(user)}
        <span class={css({ fontSize: '12px', color: 'text.muted' })}>
          {dayjs(user.createdAt).formatAsDateTime()}
        </span>
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={query.data.adminUsers.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
