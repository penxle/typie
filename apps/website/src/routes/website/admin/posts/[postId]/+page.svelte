<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import ArrowLeftIcon from '~icons/lucide/arrow-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EditIcon from '~icons/lucide/edit';
  import EyeIcon from '~icons/lucide/eye';
  import { graphql } from '$graphql';
  import { AdminIcon } from '$lib/components/admin';

  const query = graphql(`
    query AdminPost_Query($postId: String!) {
      adminPost(postId: $postId) {
        id
        title
        subtitle
        type
        contentRating
        allowComment
        allowReaction
        protectContent
        createdAt
        updatedAt
        excerpt
        password
        coverImage {
          id
          url
        }
        entity {
          id
          slug
          url
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
            postCount
            subscription {
              id
              state
            }
          }
        }
        reactionCount
        characterCount
      }
    }
  `);

  const adminEnqueuePostCompact = graphql(`
    mutation AdminPostDetail_AdminEnqueuePostCompact_Mutation($input: AdminEnqueuePostCompactInput!) {
      adminEnqueuePostCompact(input: $input)
    }
  `);
</script>

{#if $query.adminPost}
  <div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
      <div class={flex({ alignItems: 'center', gap: '12px' })}>
        <button
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            paddingX: '12px',
            paddingY: '6px',
            fontSize: '12px',
            color: 'amber.500',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'transparent',
            _hover: {
              backgroundColor: 'amber.500',
              color: 'gray.900',
            },
          })}
          onclick={() => history.back()}
          type="button"
        >
          <AdminIcon icon={ArrowLeftIcon} size={16} />
          BACK TO LIST
        </button>
        <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>POST DETAILS</h2>
      </div>
      <div class={flex({ gap: '8px' })}>
        <a
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            paddingX: '12px',
            paddingY: '6px',
            fontSize: '12px',
            color: 'amber.500',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'transparent',
            textDecoration: 'none',
            _hover: {
              backgroundColor: 'amber.500',
              color: 'gray.900',
            },
          })}
          href={$query.adminPost.entity.url}
          rel="noopener noreferrer"
          target="_blank"
        >
          <AdminIcon icon={EyeIcon} size={16} />
          PREVIEW
        </a>
        <a
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            paddingX: '12px',
            paddingY: '6px',
            fontSize: '12px',
            color: 'amber.500',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'transparent',
            textDecoration: 'none',
            _hover: {
              backgroundColor: 'amber.500',
              color: 'gray.900',
            },
          })}
          href="/{$query.adminPost.entity.slug}"
          rel="noopener noreferrer"
          target="_blank"
        >
          <AdminIcon icon={EditIcon} size={16} />
          EDIT
        </a>
      </div>
    </div>

    <div
      class={grid({
        gap: '24px',
        gridTemplateColumns: '2fr 1fr',
        alignItems: 'start',
      })}
    >
      <!-- 왼쪽 컬럼: 핵심 콘텐츠 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- CONTENT -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>CONTENT</h3>

          {#if $query.adminPost.coverImage?.url}
            <div class={css({ marginBottom: '20px' })}>
              <img
                class={css({
                  width: 'full',
                  maxHeight: '300px',
                  objectFit: 'cover',
                  borderRadius: '8px',
                })}
                alt={$query.adminPost.title}
                src={$query.adminPost.coverImage.url}
              />
            </div>
          {/if}

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>TITLE</div>
              <div class={css({ fontSize: '14px', color: 'amber.500' })}>
                {$query.adminPost.title}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>SUBTITLE</div>
              <div class={css({ fontSize: '12px', color: $query.adminPost.subtitle ? 'amber.500' : 'gray.400' })}>
                {$query.adminPost.subtitle || '(NO SUBTITLE)'}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>EXCERPT</div>
              <div
                class={css({
                  fontSize: '12px',
                  fontFamily: 'mono',
                  color: $query.adminPost.excerpt ? 'amber.500' : 'gray.400',
                  lineHeight: '[1.5]',
                })}
              >
                {$query.adminPost.excerpt || '(NO EXCERPT)'}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>CHARACTERS</div>
              <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                {comma($query.adminPost.characterCount)}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 오른쪽 컬럼: 메타데이터 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- METADATA -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>METADATA</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>POST ID</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {$query.adminPost.id}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>TYPE</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                [{$query.adminPost.type}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>STATE</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: $query.adminPost.entity.state === 'ACTIVE' ? 'green.400' : 'red.400',
                })}
              >
                [{$query.adminPost.entity.state}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CREATED</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs($query.adminPost.createdAt).formatAsDateTime()}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>UPDATED</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs($query.adminPost.updatedAt).formatAsDateTime()}
              </span>
            </div>
          </div>
        </div>

        <!-- PATH -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>PATH</h3>

          <div class={flex({ fontSize: '12px', color: 'amber.400', alignItems: 'center', gap: '4px' })}>
            {#if $query.adminPost.entity.ancestors.length > 0}
              {#each $query.adminPost.entity.ancestors as ancestor, i (ancestor.id)}
                <span>
                  {ancestor.node.__typename === 'Folder'
                    ? ancestor.node.name
                    : ancestor.node.__typename === 'Canvas'
                      ? ''
                      : ancestor.node.title}
                </span>
                {#if i < $query.adminPost.entity.ancestors.length - 1}
                  <AdminIcon icon={ChevronRightIcon} size={12} />
                {/if}
              {/each}
            {:else}
              <span class={css({ color: 'gray.500' })}>-</span>
            {/if}
          </div>
        </div>

        <!-- USER -->
        {#if $query.adminPost.entity?.user}
          <div
            class={css({
              borderWidth: '2px',
              borderColor: 'amber.500',
              padding: '24px',
              backgroundColor: 'gray.900',
            })}
          >
            <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>USER</h3>

            <div class={flex({ gap: '12px', alignItems: 'center', marginBottom: '16px' })}>
              <div
                class={css({
                  size: '40px',
                  backgroundColor: 'amber.500',
                  overflow: 'hidden',
                  flexShrink: '0',
                })}
              >
                {#if $query.adminPost.entity.user.avatar?.url}
                  <img alt={$query.adminPost.entity.user.name} src={$query.adminPost.entity.user.avatar.url} />
                {/if}
              </div>
              <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                <a
                  class={css({
                    fontSize: '14px',
                    fontWeight: 'bold',
                    color: 'amber.500',
                    _hover: { textDecoration: 'underline' },
                  })}
                  href="/admin/users/{$query.adminPost.entity.user.id}"
                >
                  {$query.adminPost.entity.user.name}
                </a>
                <div class={css({ fontSize: '11px', color: 'amber.400' })}>
                  {$query.adminPost.entity.user.email}
                </div>
              </div>
            </div>
          </div>
        {/if}

        <!-- SHARE OPTIONS -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>SHARE OPTIONS</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>VISIBILITY</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: $query.adminPost.entity.visibility === 'UNLISTED' ? 'green.400' : 'gray.400',
                })}
              >
                [{$query.adminPost.entity.visibility}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>PASSWORD</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.password ? 'amber.500' : 'gray.400' })}>
                {$query.adminPost.password || 'NONE'}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CONTENT RATING</span>
              <span
                class={css({
                  fontSize: '12px',
                  color:
                    $query.adminPost.contentRating === 'ALL'
                      ? 'green.400'
                      : $query.adminPost.contentRating === 'R15'
                        ? 'blue.400'
                        : 'red.400',
                })}
              >
                [{$query.adminPost.contentRating}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>COMMENTS</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.allowComment ? 'green.400' : 'gray.400' })}>
                {$query.adminPost.allowComment ? 'ALLOWED' : 'DISABLED'}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>REACTIONS</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.allowReaction ? 'green.400' : 'gray.400' })}>
                {$query.adminPost.allowReaction ? 'ALLOWED' : 'DISABLED'}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CONTENT COPY</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.protectContent ? 'amber.500' : 'gray.400' })}>
                {$query.adminPost.protectContent ? 'PROTECTED' : 'ALLOWED'}
              </span>
            </div>
          </div>
        </div>

        <!-- ENGAGEMENT -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>ENGAGEMENT</h3>

          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <span class={css({ fontSize: '11px', color: 'amber.400' })}>REACTIONS</span>
            <span class={css({ fontSize: '12px', color: 'amber.500' })}>
              {$query.adminPost.reactionCount}
            </span>
          </div>
        </div>

        <!-- ACTIONS -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>ACTIONS</h3>
          <button
            class={css({
              borderWidth: '1px',
              borderColor: 'amber.500',
              paddingX: '12px',
              paddingY: '8px',
              fontSize: '12px',
              color: 'amber.500',
              backgroundColor: 'transparent',
              width: 'full',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              _hover: {
                backgroundColor: 'amber.500',
                color: 'gray.900',
              },
            })}
            onclick={() =>
              adminEnqueuePostCompact({ postId: $query.adminPost.id }).then(
                () => {
                  alert('Compact Enqueue OK');
                },
                (err) => {
                  alert(`Compact Enqueue Error: ${err.message}`);
                },
              )}
            type="button"
          >
            ENQUEUE COMPACT
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
