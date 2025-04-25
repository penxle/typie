<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PencilIcon from '~icons/lucide/pencil';
  import ShareIcon from '~icons/lucide/share';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Entity from './Entity.svelte';
  import ShareFolderModal from './ShareFolderModal.svelte';
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

          site {
            id
          }
        }

        folderOption: option {
          id
          visibility
        }

        ...DashboardLayout_ShareFolderModal_folder
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

  let inputEl = $state<HTMLInputElement>();

  let open = $state(false);
  let editing = $state(false);

  let shareFolderOpen = $state(false);

  $effect(() => {
    if (editing) {
      inputEl?.select();
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
        },
        $folder.entity.depth > 0 && {
          borderLeftWidth: '1px',
          borderLeftRadius: '0',
          marginLeft: '-1px',
          paddingLeft: '14px',
          _hover: { borderLeftColor: 'gray.900' },
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
        $folder.folderOption.visibility === 'UNLISTED' && { backgroundColor: 'brand.500' },
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
            class={css(
              {
                display: 'none',
                justifyContent: 'center',
                alignItems: 'center',
                borderRadius: '4px',
                size: '16px',
                color: 'gray.400',
                opacity: '0',
                transition: 'common',
                _hover: { backgroundColor: 'gray.200' },
                _groupHover: { display: 'block', opacity: '100' },
              },
              open && { display: 'block', opacity: '100' },
            )}
          >
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        <MenuItem
          onclick={() => {
            editing = true;
          }}
        >
          <Icon icon={PencilIcon} size={12} />
          <span>폴더 이름 변경</span>
        </MenuItem>

        <MenuItem
          onclick={() => {
            shareFolderOpen = true;
          }}
        >
          <Icon icon={ShareIcon} size={12} />
          <span>폴더 공유</span>
        </MenuItem>

        {#if $folder.entity.depth < maxDepth - 1}
          <MenuItem
            onclick={async () => {
              await createFolder({
                siteId: $folder.entity.site.id,
                parentEntityId: $folder.entity.id,
                name: '새 폴더',
              });

              open = true;
            }}
          >
            <Icon icon={FolderPlusIcon} size={12} />
            <span>하위 폴더 생성</span>
          </MenuItem>
        {/if}

        <MenuItem
          onclick={async () => {
            const resp = await createPost({
              siteId: $folder.entity.site.id,
              parentEntityId: $folder.entity.id,
            });

            await goto(`/${resp.entity.slug}`);
          }}
        >
          <Icon icon={FilePlusIcon} size={12} />
          <span>하위 포스트 생성</span>
        </MenuItem>

        <MenuItem
          onclick={async () => {
            Dialog.confirm({
              title: '폴더 삭제',
              message: '정말 이 폴더를 삭제하시겠어요?',
              action: 'danger',
              actionLabel: '삭제',
              actionHandler: async () => {
                await deleteFolder({ folderId: $folder.id });
              },
            });
          }}
        >
          <Icon icon={Trash2Icon} size={12} />
          <span>폴더 삭제</span>
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

<ShareFolderModal {$folder} bind:open={shareFolderOpen} />
