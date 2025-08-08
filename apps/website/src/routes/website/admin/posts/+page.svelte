<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import SearchIcon from '~icons/lucide/search';
  import { graphql } from '$graphql';
  import { AdminIcon, AdminPagination, AdminTable } from '$lib/components/admin';
  import { QueryString, QueryStringNumber } from '$lib/state';
  import { comma } from '$lib/utils';

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const pageNumber = new QueryStringNumber('page', 1);

  const query = graphql(`
    query AdminPosts_Query($search: String, $offset: Int!, $limit: Int!) {
      adminPosts(search: $search, offset: $offset, limit: $limit) {
        totalCount

        posts {
          id
          title
          subtitle
          type
          createdAt
          updatedAt
          contentRating
          excerpt
          reactionCount
          characterCount
          entity {
            id
            slug
            visibility
            state
            ancestors {
              id
              node {
                __typename
                ... on Folder {
                  name
                }
                ... on Post {
                  title
                }
              }
            }
            user {
              id
              name
              email
              avatar {
                id
                url
              }
            }
          }

          coverImage {
            id
            url
          }
        }
      }
    }
  `);
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>POST MANAGEMENT</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'amber.400' })}>
      TOTAL POSTS: {$query.adminPosts.totalCount}
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
          placeholder="SEARCH ID, SLUG, PERMALINK OR TITLE..."
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
        { key: '$post', label: 'POST', width: '25%' },
        { key: '$id', label: 'ID', width: '10%' },
        { key: '$user', label: 'USER', width: '15%' },
        { key: '$path', label: 'PATH', width: '20%' },
        { key: '$characters', label: 'CHARACTERS', width: '10%' },
        { key: '$state', label: 'STATE', width: '10%' },
        { key: '$updatedAt', label: 'UPDATED', width: '10%' },
      ]}
      data={$query.adminPosts.posts}
      dataKey="id"
    >
      {#snippet $post(post)}
        <div class={flex({ alignItems: 'center', gap: '16px' })}>
          <div
            class={css({
              borderRadius: '8px',
              size: '40px',
              backgroundColor: 'amber.500',
              overflow: 'hidden',
            })}
          >
            {#if post.coverImage?.url}
              <img alt={post.title} src={post.coverImage.url} />
            {/if}
          </div>
          <div>
            <a
              class={css({
                fontSize: '13px',
                color: 'amber.500',
                _hover: { textDecoration: 'underline' },
              })}
              href="/admin/posts/{post.id}"
            >
              {post.title}
            </a>
            {#if post.subtitle}
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>
                {post.subtitle}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet $id(post)}
        <div class={flex({ flexDirection: 'column', gap: '2px' })}>
          <span class={css({ fontSize: '11px', color: 'gray.400' })}>
            {post.id}
          </span>
          <span class={css({ fontSize: '11px', color: 'gray.400' })}>
            {post.entity.id}
          </span>
        </div>
      {/snippet}

      {#snippet $user(post)}
        {#if post.entity?.user}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <div
              class={css({
                size: '24px',
                borderRadius: 'full',
                backgroundColor: 'amber.500',
                overflow: 'hidden',
                flexShrink: '0',
              })}
            >
              {#if post.entity.user.avatar?.url}
                <img alt={post.entity.user.name} src={post.entity.user.avatar.url} />
              {/if}
            </div>
            <div>
              <a
                class={css({
                  fontSize: '12px',
                  color: 'amber.500',
                  _hover: { textDecoration: 'underline' },
                })}
                href="/admin/users/{post.entity.user.id}"
              >
                {post.entity.user.name}
              </a>
            </div>
          </div>
        {:else}
          <span class={css({ fontSize: '12px', color: 'gray.400' })}>-</span>
        {/if}
      {/snippet}

      {#snippet $path(post)}
        <div class={flex({ fontSize: '12px', color: 'amber.400', alignItems: 'center', gap: '4px' })}>
          {#if post.entity.ancestors.length > 0}
            {#each post.entity.ancestors as ancestor, i (ancestor.id)}
              <span>
                {ancestor.node.__typename === 'Folder'
                  ? ancestor.node.name
                  : ancestor.node.__typename === 'Canvas'
                    ? ''
                    : ancestor.node.title}
              </span>
              {#if i < post.entity.ancestors.length - 1}
                <AdminIcon icon={ChevronRightIcon} size={12} />
              {/if}
            {/each}
          {:else}
            <span class={css({ color: 'gray.500' })}>-</span>
          {/if}
        </div>
      {/snippet}

      {#snippet $characters(post)}
        <span class={css({ fontSize: '12px', color: 'amber.400' })}>
          {comma(post.characterCount)} CHARS
        </span>
      {/snippet}

      {#snippet $state(post)}
        <span
          class={css({
            fontSize: '12px',
            color: post.entity.state === 'ACTIVE' ? 'green.400' : 'red.400',
          })}
        >
          {post.entity.state}
        </span>
      {/snippet}

      {#snippet $updatedAt(post)}
        <span class={css({ fontSize: '12px', color: 'amber.400' })}>
          {dayjs(post.updatedAt).formatAsDateTime()}
        </span>
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={$query.adminPosts.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
