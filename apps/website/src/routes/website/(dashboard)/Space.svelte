<script lang="ts">
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import LibraryBigIcon from '~icons/lucide/library-big';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { portal, tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntityTree from './@tree/EntityTree.svelte';
  import SidebarButton from './SidebarButton.svelte';
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

  let open = $state(false);
</script>

<SidebarButton active={open} icon={LibraryBigIcon} label="내 스페이스" onclick={() => (open = true)} />

{#if open}
  <div class={css({ position: 'fixed', inset: '0' })} onclick={() => (open = false)} role="none" use:portal></div>
{/if}

<div
  class={css({
    position: 'fixed',
    left: open ? '64px' : '59px',
    insetY: '0',
    width: '0',
    backgroundColor: 'white',
    boxShadow: 'small',
    opacity: open ? '100' : '0',
    zIndex: '50',
    transitionProperty: 'left, opacity',
    transitionDuration: '100ms',
    transitionTimingFunction: 'cubic-bezier(0.33, 1, 0.68, 1)',
    overflowX: 'hidden',
  })}
  ontransitionend={(e) => {
    if (!open) {
      e.currentTarget.style.width = '0';
    }
  }}
  ontransitionstart={(e) => {
    if (open) {
      e.currentTarget.style.width = '350px';
    }
  }}
  use:portal
>
  <div
    class={flex({
      flexDirection: 'column',
      borderRightWidth: '1px',
      borderRightColor: 'gray.100',
      borderRightRadius: '4px',
      size: 'full',
    })}
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
      <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>내 스페이스</span>

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

    <div class={css({ paddingX: '16px', overflowY: 'auto' })}>
      <EntityTree {$site} />
    </div>
  </div>
</div>
