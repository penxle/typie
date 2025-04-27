<script lang="ts">
  import { fly } from 'svelte/transition';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import LibraryBigIcon from '~icons/lucide/library-big';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
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
  <div class={css({ position: 'fixed', inset: '0', zIndex: '50' })}>
    <div class={css({ position: 'absolute', inset: '0' })} onclick={() => (open = false)} role="none"></div>

    <div
      class={flex({
        position: 'absolute',
        left: '64px',
        insetY: '0',
        flexDirection: 'column',
        borderRightWidth: '1px',
        borderColor: 'gray.100',
        borderRightRadius: '4px',
        width: '350px',
        backgroundColor: 'white',
        boxShadow: 'small',
        zIndex: '1',
      })}
      transition:fly={{ x: -5, duration: 100 }}
    >
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          flexShrink: '0',
          gap: '4px',
          borderBottomWidth: '1px',
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'white',
          zIndex: '1',
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
{/if}
