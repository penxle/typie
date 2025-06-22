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

          coverImage {
            id
            url
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
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>POST MANAGEMENT</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', fontFamily: 'mono', color: 'amber.400' })}>
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
          placeholder="SEARCH TITLE OR SUBTITLE..."
          type="text"
          bind:value={searchQuery.current}
        />
      </div>
    </div>

    <AdminTable
      columns={[
        { key: '$post', label: '포스트', width: '45%' },
        { key: '$type', label: '타입', width: '15%' },
        { key: '$contentRating', label: '등급', width: '15%' },
        { key: '$updatedAt', label: '수정일', width: '25%' },
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
              <div class={css({ fontSize: '11px', fontFamily: 'mono', color: 'amber.400' })}>
                {post.subtitle}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet $type(post)}
        <span class={css({ fontSize: '12px', color: post.type === 'NORMAL' ? 'amber.500' : 'gray.400' })}>
          [{post.type}]
        </span>
      {/snippet}

      {#snippet $contentRating(post)}
        <span
          class={css({
            fontSize: '12px',
            color:
              post.contentRating === 'ALL'
                ? 'green.400'
                : post.contentRating === 'R15'
                  ? 'blue.400'
                  : post.contentRating === 'R19'
                    ? 'red.400'
                    : 'gray.400',
          })}
        >
          [{post.contentRating}]
        </span>
      {/snippet}

      {#snippet $updatedAt(post)}
        {dayjs(post.updatedAt).formatAsDateTime()}
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={$query.adminPosts.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
