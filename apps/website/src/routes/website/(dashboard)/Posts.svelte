<script lang="ts">
  import { TypieError } from '@/errors';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import MoveUpRightIcon from '~icons/lucide/move-up-right';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto, pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { portal, tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntityTree from './@tree/EntityTree.svelte';
  import type { DashboardLayout_Posts_site, DashboardLayout_Posts_user } from '$graphql';

  type Props = {
    $site: DashboardLayout_Posts_site;
    $user: DashboardLayout_Posts_user;
  };

  let { $site: _site, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Posts_user on User {
        id

        usage {
          postCount
        }

        planRule {
          maxPostCount
        }

        plan {
          id
        }
      }
    `),
  );

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Posts_site on Site {
        id

        ...DashboardLayout_EntityTree_site
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Posts_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const createFolder = graphql(`
    mutation DashboardLayout_Posts_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();
</script>

{#if app.state.postsOpen && !app.preference.current.postsExpanded}
  <div
    class={css({ position: 'fixed', inset: '0', zIndex: '40' })}
    onclick={() => (app.state.postsOpen = false)}
    role="none"
    use:portal
  ></div>
{/if}

<div
  style:--min-width="240px"
  style:--width="15vw"
  style:--max-width="300px"
  style:--overflow="hidden"
  class={css(
    {
      flexShrink: '0',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
    },
    app.preference.current.postsExpanded
      ? {
          position: 'relative',
          marginY: '8px',
          marginRight: app.preference.current.postsExpanded === 'open' ? '4px' : '0',
          minWidth: app.preference.current.postsExpanded === 'open' ? 'var(--min-width)' : '0',
          maxWidth: app.preference.current.postsExpanded === 'open' ? 'var(--max-width)' : '0',
          opacity: app.preference.current.postsExpanded === 'open' ? '100' : '0',
          transitionProperty: 'min-width, max-width, opacity, position, margin-block',
        }
      : {
          position: 'fixed',
          left: app.state.postsOpen ? '64px' : '59px',
          insetY: '0',
          minWidth: app.state.postsOpen ? 'var(--min-width)' : '0',
          width: app.state.postsOpen ? 'var(--fixed-width, 0)' : '0',
          maxWidth: app.state.postsOpen ? 'var(--max-width)' : '0',
          opacity: app.state.postsOpen ? '100' : '0',
          zIndex: '50',
          transitionProperty: 'left, opacity, position, margin-block',
          overflow: 'var(--overflow)',
        },
  )}
  ontransitionendcapture={(e) => {
    if (!app.preference.current.postsExpanded && !app.state.postsOpen) {
      e.currentTarget.style.setProperty('--fixed-width', '0');
      e.currentTarget.style.setProperty('--overflow', 'hidden');
    }
  }}
  ontransitionstartcapture={(e) => {
    if (!app.preference.current.postsExpanded && app.state.postsOpen) {
      e.currentTarget.style.setProperty('--fixed-width', 'var(--width)');
      e.currentTarget.style.setProperty('--overflow', 'visible');
    }
  }}
>
  <div
    class={css(
      {
        display: 'flex',
        flexDirection: 'column',
        minWidth: 'var(--min-width)',
        width: 'var(--width)',
        maxWidth: 'var(--max-width)',
        height: 'full',
        backgroundColor: 'white',
        transitionProperty: 'border, border-radius, box-shadow',
        transitionDuration: '150ms',
        transitionTimingFunction: 'ease',
        overflow: 'hidden',
      },
      app.preference.current.postsExpanded
        ? {
            borderWidth: '[0.5px]',
            borderRadius: '4px',
            boxShadow: '[0 3px 6px -2px {colors.gray.950/3}, 0 1px 1px {colors.gray.950/5}]',
          }
        : {
            borderColor: 'gray.100',
            borderRightWidth: '1px',
            borderRightRadius: '4px',
            boxShadow: 'small',
          },
    )}
  >
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        flexShrink: '0',
        gap: '4px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'white',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>내 포스트</span>

        <button
          class={center({
            borderRadius: '4px',
            size: '20px',
            color: 'gray.500',
            transition: 'common',
            _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
          })}
          onclick={() => {
            if (app.preference.current.postsExpanded) {
              app.state.postsOpen = app.preference.current.postsExpanded === 'open';
              app.preference.current.postsExpanded = false;
            } else {
              app.preference.current.postsExpanded = app.state.postsOpen ? 'open' : 'closed';
            }
          }}
          type="button"
          use:tooltip={{ message: app.preference.current.postsExpanded ? '패널 고정 해제' : '패널 고정' }}
        >
          <Icon icon={app.preference.current.postsExpanded ? PanelLeftDashedIcon : PanelLeftIcon} size={14} />
        </button>
      </div>

      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'gray.500',
            transition: 'common',
            _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
          })}
          onclick={async () => {
            await createFolder({
              siteId: $site.id,
              name: '새 폴더',
            });
          }}
          type="button"
          use:tooltip={{ message: '새 폴더 생성' }}
        >
          <Icon icon={FolderPlusIcon} />
        </button>

        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'gray.500',
            transition: 'common',
            _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
          })}
          onclick={async () => {
            try {
              const resp = await createPost({
                siteId: $site.id,
              });

              await goto(`/${resp.entity.slug}`);
            } catch (err) {
              if (err instanceof TypieError && err.code === 'max_post_count_reached') {
                pushState('', { shallowRoute: '/preference/billing' });
              }
            }
          }}
          type="button"
          use:tooltip={{ message: '새 포스트 생성' }}
        >
          <Icon icon={SquarePenIcon} />
        </button>
      </div>
    </div>

    <div
      class={css({
        flexGrow: '1',
        paddingX: '16px',
        paddingBottom: '32px',
        scrollPaddingY: '16px',
        overflowY: 'auto',
      })}
    >
      <EntityTree {$site} />
    </div>

    {#if !$user.plan}
      {@const usage = ($user.planRule.maxPostCount - $user.usage.postCount) * 100}

      <div class={css({ paddingX: '16px', paddingY: '20px' })}>
        <button
          class={css({
            borderWidth: '1px',
            borderColor: 'gray.100',
            borderRadius: '6px',
            textAlign: 'left',
            paddingX: '10px',
            paddingY: '8px',
            width: 'full',
            boxShadow: 'small',
            _hover: { backgroundColor: 'gray.100' },
          })}
          onclick={() => {
            pushState('', { shallowRoute: '/preference/billing' });
          }}
          type="button"
        >
          <p class={css({ fontSize: '12px', color: 'gray.600' })}>현재 사용량</p>

          <div class={css({ position: 'relative', marginY: '4px' })}>
            <div
              style:width={`calc(100% - ${usage}%)`}
              class={css({
                position: 'absolute',
                top: '0',
                left: '0',
                borderRadius: 'full',
                backgroundColor: 'brand.500',
                maxWidth: 'full',
                height: '4px',
              })}
            ></div>
            <div class={css({ borderRadius: 'full', backgroundColor: 'gray.200', height: '4px' })}></div>
          </div>

          <div class={css({ fontSize: '12px', color: 'gray.600', textAlign: 'right' })}>
            {$user.usage.postCount} / {$user.planRule.maxPostCount}
          </div>

          <div
            class={flex({
              align: 'center',
              justify: 'center',
              gap: '4px',
              marginTop: '4px',
              borderWidth: '1px',
              borderColor: 'gray.200',
              borderRadius: '4px',
              paddingX: '12px',
              paddingY: '6px',
              fontSize: '13px',
              fontWeight: 'semibold',
              textAlign: 'center',
              color: 'gray.700',
            })}
          >
            플랜 업그레이드
            <Icon style={css.raw({ color: 'gray.400' })} icon={MoveUpRightIcon} size={12} />
          </div>
        </button>
      </div>
    {/if}
  </div>
</div>
