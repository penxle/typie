<script lang="ts">
  import dayjs from 'dayjs';
  import ArrowLeftIcon from '~icons/lucide/arrow-left';
  import EditIcon from '~icons/lucide/edit';
  import EyeIcon from '~icons/lucide/eye';
  import { graphql } from '$graphql';
  import { AdminIcon } from '$lib/components/admin';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';

  const query = graphql(`
    query AdminPost_Query($postId: String!) {
      adminPost(postId: $postId) {
        id
        title
        subtitle
        type
        maxWidth
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
</script>

{#if $query.adminPost}
  <div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
      <div class={flex({ alignItems: 'center', gap: '12px' })}>
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
            textDecoration: 'none',
            _hover: {
              backgroundColor: 'amber.500',
              color: 'gray.900',
            },
          })}
          href="/admin/posts"
        >
          <AdminIcon icon={ArrowLeftIcon} size={16} />
          BACK TO LIST
        </a>
        <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>POST DETAILS</h2>
      </div>
      <div class={flex({ gap: '8px' })}>
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
          type="button"
        >
          <AdminIcon icon={EyeIcon} size={16} />
          PREVIEW
        </button>
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
          type="button"
        >
          <AdminIcon icon={EditIcon} size={16} />
          EDIT
        </button>
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

            <div class={grid({ gridTemplateColumns: '1fr 1fr', gap: '16px', marginTop: '8px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>TYPE</div>
                <span class={css({ fontSize: '12px', color: $query.adminPost.type === 'NORMAL' ? 'amber.500' : 'gray.400' })}>
                  [{$query.adminPost.type}]
                </span>
              </div>

              <div>
                <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>CONTENT RATING</div>
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
            </div>
          </div>
        </div>

        <!-- AUTHOR -->
        {#if $query.adminPost.entity?.user}
          <div
            class={css({
              borderWidth: '2px',
              borderColor: 'amber.500',
              padding: '24px',
              backgroundColor: 'gray.900',
            })}
          >
            <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>AUTHOR</h3>

            <div class={flex({ gap: '16px', marginBottom: '20px' })}>
              <div
                class={css({
                  size: '64px',
                  backgroundColor: 'amber.500',
                  overflow: 'hidden',
                  flexShrink: '0',
                })}
              >
                {#if $query.adminPost.entity.user.avatar?.url}
                  <img alt={$query.adminPost.entity.user.name} src={$query.adminPost.entity.user.avatar.url} />
                {/if}
              </div>
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                <a
                  class={css({
                    fontSize: '16px',
                    fontWeight: 'bold',
                    color: 'amber.500',
                    _hover: { textDecoration: 'underline' },
                  })}
                  href="/admin/users/{$query.adminPost.entity.user.id}"
                >
                  {$query.adminPost.entity.user.name}
                </a>
                <div class={css({ fontSize: '12px', color: 'amber.400' })}>
                  {$query.adminPost.entity.user.email}
                </div>
                <div class={flex({ gap: '16px' })}>
                  <span class={css({ fontSize: '12px', color: 'amber.400' })}>
                    {$query.adminPost.entity.user.postCount} POSTS
                  </span>
                  {#if $query.adminPost.entity.user.subscription}
                    <span class={css({ fontSize: '12px', color: 'green.400' })}>
                      [{$query.adminPost.entity.user.subscription.state}]
                    </span>
                  {:else}
                    <span class={css({ fontSize: '12px', color: 'gray.400' })}>[FREE]</span>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {/if}

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

          <div class={grid({ gridTemplateColumns: 'repeat(2, 1fr)', gap: '24px' })}>
            <div>
              <div class={css({ fontSize: '24px', color: 'amber.500', marginBottom: '4px' })}>
                {$query.adminPost.reactionCount}
              </div>
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>REACTIONS</div>
            </div>

            <div>
              <div class={css({ fontSize: '24px', color: 'amber.500', marginBottom: '4px' })}>
                {comma($query.adminPost.characterCount)}
              </div>
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>CHARACTERS</div>
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
            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>POST ID</div>
              <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                {$query.adminPost.id}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>CREATED</div>
              <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs($query.adminPost.createdAt).formatAsDateTime()}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>UPDATED</div>
              <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs($query.adminPost.updatedAt).formatAsDateTime()}
              </div>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>MAX WIDTH</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {$query.adminPost.maxWidth}PX
              </span>
            </div>
          </div>
        </div>

        <!-- PERMISSIONS -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>PERMISSIONS</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
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
          </div>
        </div>

        <!-- SECURITY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'amber.500',
            padding: '24px',
            backgroundColor: 'gray.900',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'amber.500', marginBottom: '20px' })}>SECURITY</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>PASSWORD</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.password ? 'red.400' : 'green.400' })}>
                {$query.adminPost.password ? 'PROTECTED' : 'PUBLIC'}
              </span>
            </div>

            {#if $query.adminPost.password}
              <div class={css({ fontSize: '10px', color: 'amber.400' })}>
                {'*'.repeat($query.adminPost.password.length)} ({$query.adminPost.password.length} CHARS)
              </div>
            {/if}

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CONTENT COPY</span>
              <span class={css({ fontSize: '12px', color: $query.adminPost.protectContent ? 'amber.500' : 'gray.400' })}>
                {$query.adminPost.protectContent ? 'PROTECTED' : 'ALLOWED'}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
