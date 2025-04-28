<script lang="ts">
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { portal, tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntityTree from './@tree/EntityTree.svelte';
  import type { DashboardLayout_Space_site } from '$graphql';

  type Props = {
    $site: DashboardLayout_Space_site;
  };

  let { $site: _site }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Space_site on Site {
        id

        ...DashboardLayout_EntityTree_site
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Space_CreatePost_Mutation($input: CreatePostInput!) {
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
    mutation DashboardLayout_Space_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();
</script>

{#if app.state.spaceOpen && !app.preference.current.spaceExpanded}
  <div
    class={css({ position: 'fixed', inset: '0', zIndex: '40' })}
    onclick={() => (app.state.spaceOpen = false)}
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
    app.preference.current.spaceExpanded
      ? {
          position: 'relative',
          marginY: '8px',
          marginRight: app.preference.current.spaceExpanded === 'open' ? '4px' : '0',
          minWidth: app.preference.current.spaceExpanded === 'open' ? 'var(--min-width)' : '0',
          maxWidth: app.preference.current.spaceExpanded === 'open' ? 'var(--max-width)' : '0',
          opacity: app.preference.current.spaceExpanded === 'open' ? '100' : '0',
          transitionProperty: 'min-width, max-width, opacity, position, margin-block',
        }
      : {
          position: 'fixed',
          left: app.state.spaceOpen ? '64px' : '59px',
          insetY: '0',
          minWidth: app.state.spaceOpen ? 'var(--min-width)' : '0',
          width: app.state.spaceOpen ? 'var(--fixed-width, 0)' : '0',
          maxWidth: app.state.spaceOpen ? 'var(--max-width)' : '0',
          opacity: app.state.spaceOpen ? '100' : '0',
          zIndex: '50',
          transitionProperty: 'left, opacity, position, margin-block',
          overflow: 'var(--overflow)',
        },
  )}
  ontransitionendcapture={(e) => {
    if (!app.preference.current.spaceExpanded && !app.state.spaceOpen) {
      e.currentTarget.style.setProperty('--fixed-width', '0');
      e.currentTarget.style.setProperty('--overflow', 'hidden');
    }
  }}
  ontransitionstartcapture={(e) => {
    if (!app.preference.current.spaceExpanded && app.state.spaceOpen) {
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
      app.preference.current.spaceExpanded
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
        <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>내 스페이스</span>

        <button
          class={center({
            borderRadius: '4px',
            size: '20px',
            color: 'gray.500',
            transition: 'common',
            _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
          })}
          onclick={() => {
            if (app.preference.current.spaceExpanded) {
              app.state.spaceOpen = app.preference.current.spaceExpanded === 'open';
              app.preference.current.spaceExpanded = false;
            } else {
              app.preference.current.spaceExpanded = app.state.spaceOpen ? 'open' : 'closed';
            }
          }}
          type="button"
          use:tooltip={{ message: app.preference.current.spaceExpanded ? '패널 고정 해제' : '패널 고정' }}
        >
          <Icon icon={app.preference.current.spaceExpanded ? PanelLeftDashedIcon : PanelLeftIcon} size={14} />
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
            const resp = await createPost({
              siteId: $site.id,
            });

            await goto(`/${resp.entity.slug}`);
          }}
          type="button"
          use:tooltip={{ message: '새 포스트 생성' }}
        >
          <Icon icon={SquarePenIcon} />
        </button>
      </div>
    </div>

    <div class={css({ paddingX: '16px', scrollPaddingY: '16px', overflowY: 'auto' })}>
      <EntityTree {$site} />
    </div>
  </div>
</div>
