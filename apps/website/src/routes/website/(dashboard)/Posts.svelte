<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto, pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { portal, tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { comma, formatBytes } from '$lib/utils';
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

        planRule {
          maxTotalCharacterCount
          maxTotalBlobSize
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

        usage {
          totalCharacterCount
          totalBlobSize
        }

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

  const totalCharacterCountProgress = $derived.by(() => {
    if ($user.planRule.maxTotalCharacterCount === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalCharacterCount / $user.planRule.maxTotalCharacterCount);
  });

  const totalBlobSizeProgress = $derived.by(() => {
    if ($user.planRule.maxTotalBlobSize === -1) {
      return -1;
    }

    return Math.min(1, $site.usage.totalBlobSize / $user.planRule.maxTotalBlobSize);
  });

  $effect(() => {
    app.state.progress.totalCharacterCount = totalCharacterCountProgress;
    app.state.progress.totalBlobSize = totalBlobSizeProgress;
  });
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
              mixpanel.track('toggle_posts_expanded', { expanded: false });
            } else {
              app.preference.current.postsExpanded = app.state.postsOpen ? 'open' : 'closed';
              mixpanel.track('toggle_posts_expanded', { expanded: true });
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
            mixpanel.track('create_folder', { via: 'tree' });
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
            const resp = await createPost({
              siteId: $site.id,
            });

            mixpanel.track('create_post', { via: 'tree' });

            await goto(`/${resp.entity.slug}`);
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
        paddingTop: '8px',
        paddingBottom: '32px',
        scrollPaddingY: '16px',
        overflowY: 'auto',
      })}
    >
      <EntityTree {$site} />
    </div>

    {#if totalCharacterCountProgress !== -1 || totalBlobSizeProgress !== -1}
      <button
        class={flex({
          flexDirection: 'column',
          gap: '8px',
          borderTopWidth: '1px',
          borderColor: 'gray.100',
          paddingX: '12px',
          paddingTop: '12px',
          paddingBottom: '20px',
          transitionProperty: 'background-color',
          transitionDuration: '250ms',
          transitionTimingFunction: 'ease',
          _hover: { backgroundColor: 'gray.50' },
        })}
        onclick={() => {
          pushState('', { shallowRoute: '/preference/billing' });
          mixpanel.track('open_billing', { via: 'usage_widget' });
        }}
        type="button"
      >
        <div class={flex({ flexDirection: 'column', gap: '2px' })}>
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '2px' })}>
            <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>글자 수</div>

            <div class={css({ fontSize: '12px', color: 'gray.500' })}>
              {comma($site.usage.totalCharacterCount)}자 / {comma($user.planRule.maxTotalCharacterCount)}자
            </div>
          </div>

          <div class={css({ position: 'relative', borderRadius: 'full', height: '4px', overflow: 'hidden' })}>
            <div
              style:width={`${totalCharacterCountProgress * 100}%`}
              class={css({
                position: 'absolute',
                left: '0',
                insetY: '0',
                borderRightRadius: 'full',
                backgroundColor: 'brand.400',
                maxWidth: 'full',
                transitionProperty: 'width',
                transitionDuration: '150ms',
                transitionTimingFunction: 'ease',
              })}
            ></div>

            <div class={css({ backgroundColor: 'gray.200', height: 'full' })}></div>
          </div>
        </div>

        <div class={flex({ flexDirection: 'column', gap: '2px' })}>
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'gray.700' })}>첨부 파일</div>

            <div class={css({ fontSize: '12px', color: 'gray.500' })}>
              {formatBytes($site.usage.totalBlobSize)} / {formatBytes($user.planRule.maxTotalBlobSize)}
            </div>
          </div>

          <div class={css({ position: 'relative', borderRadius: 'full', height: '4px', overflow: 'hidden' })}>
            <div
              style:width={`${totalBlobSizeProgress * 100}%`}
              class={css({
                position: 'absolute',
                left: '0',
                insetY: '0',
                borderRightRadius: 'full',
                backgroundColor: 'brand.400',
                maxWidth: 'full',
                transitionProperty: 'width',
                transitionDuration: '150ms',
                transitionTimingFunction: 'ease',
              })}
            ></div>

            <div class={css({ backgroundColor: 'gray.200', height: 'full' })}></div>
          </div>
        </div>
      </button>
    {/if}
  </div>
</div>
