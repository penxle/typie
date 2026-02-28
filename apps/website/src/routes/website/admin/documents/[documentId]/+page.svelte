<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import ArrowLeftIcon from '~icons/lucide/arrow-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EditIcon from '~icons/lucide/edit';
  import EyeIcon from '~icons/lucide/eye';
  import { AdminIcon } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
</script>

{#if query.data.adminDocument}
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
        <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>DOCUMENT DETAILS</h2>
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
          href={query.data.adminDocument.entity.url}
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
          href="/{query.data.adminDocument.entity.slug}"
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

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>TITLE</div>
              <div class={css({ fontSize: '14px', color: 'amber.500' })}>
                {query.data.adminDocument.title}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>SUBTITLE</div>
              <div class={css({ fontSize: '12px', color: query.data.adminDocument.subtitle ? 'amber.500' : 'gray.400' })}>
                {query.data.adminDocument.subtitle || '(NO SUBTITLE)'}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>EXCERPT</div>
              <div
                class={css({
                  fontSize: '12px',
                  fontFamily: 'mono',
                  color: query.data.adminDocument.excerpt ? 'amber.500' : 'gray.400',
                  lineHeight: '[1.5]',
                })}
              >
                {query.data.adminDocument.excerpt || '(NO EXCERPT)'}
              </div>
            </div>

            <div>
              <div class={css({ fontSize: '11px', color: 'amber.400', marginBottom: '4px' })}>CHARACTERS</div>
              <div class={css({ fontSize: '12px', color: 'amber.500' })}>
                {comma(query.data.adminDocument.characterCount)}
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
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>DOCUMENT ID</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {query.data.adminDocument.id}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>TYPE</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                [{query.data.adminDocument.type}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>STATE</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: query.data.adminDocument.entity.state === 'ACTIVE' ? 'green.400' : 'red.400',
                })}
              >
                [{query.data.adminDocument.entity.state}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CREATED</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs(query.data.adminDocument.createdAt).formatAsDateTime()}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>UPDATED</span>
              <span class={css({ fontSize: '12px', color: 'amber.500' })}>
                {dayjs(query.data.adminDocument.updatedAt).formatAsDateTime()}
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
            {#if query.data.adminDocument.entity.ancestors.length > 0}
              {#each query.data.adminDocument.entity.ancestors as ancestor, i (ancestor.id)}
                <span>
                  {ancestor.node.__typename === 'Folder' ? ancestor.node.name : ancestor.node.title}
                </span>
                {#if i < query.data.adminDocument.entity.ancestors.length - 1}
                  <AdminIcon icon={ChevronRightIcon} size={12} />
                {/if}
              {/each}
            {:else}
              <span class={css({ color: 'gray.500' })}>-</span>
            {/if}
          </div>
        </div>

        <!-- USER -->
        {#if query.data.adminDocument.entity?.user}
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
                {#if query.data.adminDocument.entity.user.avatar?.url}
                  <img alt={query.data.adminDocument.entity.user.name} src={query.data.adminDocument.entity.user.avatar.url} />
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
                  href="/admin/users/{query.data.adminDocument.entity.user.id}"
                >
                  {query.data.adminDocument.entity.user.name}
                </a>
                <div class={css({ fontSize: '11px', color: 'amber.400' })}>
                  {query.data.adminDocument.entity.user.email}
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
                  color:
                    query.data.adminDocument.entity.visibility === 'UNLISTED'
                      ? 'green.400'
                      : query.data.adminDocument.entity.visibility === 'PUBLIC'
                        ? 'blue.400'
                        : 'gray.400',
                })}
              >
                [{query.data.adminDocument.entity.visibility}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>PASSWORD</span>
              <span class={css({ fontSize: '12px', color: query.data.adminDocument.password ? 'amber.500' : 'gray.400' })}>
                {query.data.adminDocument.password || 'NONE'}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CONTENT RATING</span>
              <span
                class={css({
                  fontSize: '12px',
                  color:
                    query.data.adminDocument.contentRating === 'ALL'
                      ? 'green.400'
                      : query.data.adminDocument.contentRating === 'R15'
                        ? 'blue.400'
                        : 'red.400',
                })}
              >
                [{query.data.adminDocument.contentRating}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>REACTIONS</span>
              <span class={css({ fontSize: '12px', color: query.data.adminDocument.allowReaction ? 'green.400' : 'gray.400' })}>
                {query.data.adminDocument.allowReaction ? 'ALLOWED' : 'DISABLED'}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'amber.400' })}>CONTENT COPY</span>
              <span class={css({ fontSize: '12px', color: query.data.adminDocument.protectContent ? 'amber.500' : 'gray.400' })}>
                {query.data.adminDocument.protectContent ? 'PROTECTED' : 'ALLOWED'}
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
              {query.data.adminDocument.reactionCount}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
