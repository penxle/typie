<script lang="ts">
  import { EntityType, EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PencilIcon from '~icons/lucide/pencil-line';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Menu, MenuItem, RingSpinner } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Entity from './Entity.svelte';
  import { maxDepth } from './utils';
  import type { DashboardLayout_EntityTree_Folder_entity, DashboardLayout_EntityTree_Folder_folder, List } from '$graphql';

  type Props = {
    $folder: DashboardLayout_EntityTree_Folder_folder;
    $entities: List<DashboardLayout_EntityTree_Folder_entity>;
  };

  let { $folder: _folder, $entities: _entities }: Props = $props();

  const folder = fragment(
    _folder,
    graphql(`
      fragment DashboardLayout_EntityTree_Folder_folder on Folder {
        id
        name

        entity {
          id
          order
          depth
          visibility

          site {
            id
          }
        }
      }
    `),
  );

  const entities = fragment(
    _entities,
    graphql(`
      fragment DashboardLayout_EntityTree_Folder_entity on Entity {
        id

        ...DashboardLayout_EntityTree_Entity_entity
      }
    `),
  );

  const descendants = graphql(`
    query DashboardLayout_EntityTree_Folder_Descendants_Query($entityId: ID!) @client {
      entity(entityId: $entityId) {
        id

        descendants {
          id
          type
        }
      }
    }
  `);

  const createPost = graphql(`
    mutation DashboardLayout_EntityTree_Folder_CreatePost_Mutation($input: CreatePostInput!) {
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
    mutation DashboardLayout_EntityTree_Folder_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const renameFolder = graphql(`
    mutation DashboardLayout_EntityTree_Folder_RenameFolder_Mutation($input: RenameFolderInput!) {
      renameFolder(input: $input) {
        id
        name
      }
    }
  `);

  const deleteFolder = graphql(`
    mutation DashboardLayout_EntityTree_Folder_DeleteFolder_Mutation($input: DeleteFolderInput!) {
      deleteFolder(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();
  const active = $derived(app.state.ancestors.includes($folder.entity.id));

  let inputEl = $state<HTMLInputElement>();

  let open = $state(false);
  let editing = $state(false);
  let loadingDescendants = $state(false);

  $effect(() => {
    if (editing) {
      inputEl?.select();
    }
  });

  $effect.pre(() => {
    if (active) {
      open = true;
    }
  });
</script>

<details data-depth={$folder.entity.depth} data-id={$folder.entity.id} data-order={$folder.entity.order} data-type="folder" bind:open>
  <summary
    class={cx(
      'group',
      css(
        {
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          cursor: 'pointer',
          _hover: { backgroundColor: 'gray.100' },
          '&:has([aria-pressed="true"])': { backgroundColor: 'gray.100' },
        },
        $folder.entity.depth > 0 && {
          borderLeftWidth: '1px',
          borderLeftRadius: '0',
          marginLeft: '-1px',
          paddingLeft: '14px',
          _hover: { borderLeftColor: 'gray.300' },
        },
      ),
    )}
    aria-selected="false"
    data-anchor={$entities.length > 0}
    onkeyup={(e) => {
      if (e.code === 'Space') {
        e.preventDefault();
      }
    }}
    role="treeitem"
  >
    <div
      class={css(
        { flex: 'none', borderRadius: 'full', backgroundColor: 'gray.200', size: '4px' },
        $folder.entity.visibility === EntityVisibility.UNLISTED && { backgroundColor: 'brand.500' },
      )}
    ></div>

    <Icon style={css.raw({ color: 'gray.500' })} icon={open ? ChevronDownIcon : ChevronRightIcon} size={14} />

    {#if editing}
      <form
        class={css({ display: 'contents' })}
        onsubmit={async (e) => {
          e.preventDefault();

          const formData = new FormData(e.currentTarget);

          await renameFolder({
            folderId: $folder.id,
            name: formData.get('name') as string,
          });

          editing = false;
        }}
      >
        <input
          bind:this={inputEl}
          name="name"
          class={css({
            flexGrow: '1',
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'gray.600',
            minWidth: '0',
          })}
          defaultValue={$folder.name}
          onblur={(e) => e.currentTarget.form?.requestSubmit()}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              e.currentTarget.form?.reset();
              editing = false;
            }
          }}
        />
      </form>
    {:else}
      <span
        class={css({
          flexGrow: '1',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'gray.600',
          wordBreak: 'break-all',
          lineClamp: '1',
        })}
      >
        {$folder.name}
      </span>

      <Menu placement="bottom-start">
        {#snippet button({ open })}
          <div
            class={center({
              borderRadius: '4px',
              size: '16px',
              color: 'gray.400',
              opacity: '0',
              transition: 'common',
              _hover: { backgroundColor: 'gray.200' },
              _groupHover: { opacity: '100' },
              _pressed: { backgroundColor: 'gray.200', opacity: '100' },
            })}
            aria-pressed={open}
          >
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        <MenuItem icon={PencilIcon} onclick={() => (editing = true)}>이름 변경</MenuItem>
        <MenuItem icon={BlendIcon} onclick={() => (app.state.shareOpen = $folder.entity.id)}>공유</MenuItem>

        <HorizontalDivider color="secondary" />

        <MenuItem
          icon={SquarePenIcon}
          onclick={async () => {
            const resp = await createPost({
              siteId: $folder.entity.site.id,
              parentEntityId: $folder.entity.id,
            });

            await goto(`/${resp.entity.slug}`);
          }}
        >
          하위 포스트 생성
        </MenuItem>

        {#if $folder.entity.depth < maxDepth - 1}
          <MenuItem
            icon={FolderPlusIcon}
            onclick={async () => {
              await createFolder({
                siteId: $folder.entity.site.id,
                parentEntityId: $folder.entity.id,
                name: '새 폴더',
              });

              open = true;
            }}
          >
            하위 폴더 생성
          </MenuItem>
        {/if}

        <HorizontalDivider color="secondary" />

        <MenuItem
          icon={TrashIcon}
          onclick={async () => {
            loadingDescendants = true;
            descendants.load({ entityId: $folder.entity.id }).then(() => {
              loadingDescendants = false;
            });

            Dialog.confirm({
              title: '폴더 삭제',
              message: '정말 이 폴더를 삭제하시겠어요?',
              children: descendantsView,
              action: 'danger',
              actionLabel: '삭제',
              actionHandler: async () => {
                await deleteFolder({ folderId: $folder.id });
              },
            });
          }}
          variant="danger"
        >
          삭제
        </MenuItem>
      </Menu>
    {/if}
  </summary>

  <div class={flex({ flexDirection: 'column', borderLeftWidth: '1px', marginLeft: '24px' })} aria-hidden={!open} role="tree">
    {#each $entities as entity (entity.id)}
      <Entity $entity={entity} />
    {:else}
      <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'gray.400' })}>
        폴더가 비어있어요
      </div>
    {/each}
  </div>
</details>

{#snippet descendantsView()}
  {#if !$descendants || loadingDescendants}
    <div
      class={flex({ alignItems: 'center', gap: '6px', borderRadius: '8px', paddingX: '12px', paddingY: '8px', backgroundColor: 'gray.50' })}
    >
      <RingSpinner style={css.raw({ size: '13px', color: 'gray.500' })} />
      <span class={css({ fontSize: '13px', color: 'gray.500' })}>함께 삭제될 폴더와 포스트 계산중...</span>
    </div>
  {:else}
    {@const folders = $descendants.entity.descendants.filter((d) => d.type === EntityType.FOLDER).length}
    {@const posts = $descendants.entity.descendants.filter((d) => d.type === EntityType.POST).length}

    {#if folders > 0 || posts > 0}
      <div
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'red.50',
        })}
      >
        <Icon style={css.raw({ color: 'red.600' })} icon={TriangleAlertIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'red.600' })}>
          {#if folders > 0 && posts > 0}
            {folders}개의 하위 폴더와 {posts}개의 하위 포스트가 함께 삭제돼요
          {:else if folders > 0}
            {folders}개의 하위 폴더가 함께 삭제돼요
          {:else if posts > 0}
            {posts}개의 하위 포스트가 함께 삭제돼요
          {/if}
        </span>
      </div>
    {:else}
      <div
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'green.50',
        })}
      >
        <Icon style={css.raw({ color: 'green.600' })} icon={CheckIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'green.600' })}>비어있는 폴더에요</span>
      </div>
    {/if}
  {/if}
{/snippet}
