<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import CopyIcon from '~icons/lucide/copy';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PencilIcon from '~icons/lucide/pencil';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import PageList from './PageList.svelte';
  import type { Entity, RootEntity } from './types';

  type Props = {
    entity: Entity;
    depth: number;
    onPointerDown: (e: PointerEvent, entity: Entity) => void;
    registerNode: (node: HTMLElement | undefined, entity: Entity & { depth: number }) => void;
    siteId: string;
    nodeMap: Map<HTMLElement, (Entity | RootEntity) & { depth: number }>;
  };

  let { entity, depth, onPointerDown, registerNode, siteId, nodeMap }: Props = $props();

  let open = $state(false);
  let itemEl = $state<HTMLElement>();

  let editing = $state(false);
  let inputEl = $state<HTMLInputElement>();
  let name = $state('');

  $effect(() => {
    if (entity.node?.__typename === 'Folder') name = entity.node.name;
  });

  $effect(() => {
    if (itemEl) registerNode(itemEl, { ...entity, depth });
  });

  $effect(() => {
    if (editing && inputEl) {
      inputEl.select();
    }
  });

  const createPost = graphql(`
    mutation DashboardLayout_PageItem_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const duplicatePost = graphql(`
    mutation DashboardLayout_PageItem_DuplicatePost_Mutation($input: DuplicatePostInput!) {
      duplicatePost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deletePost = graphql(`
    mutation DashboardLayout_PageItem_DeletePost_Mutation($input: DeletePostInput!) {
      deletePost(input: $input) {
        id
      }
    }
  `);

  const createFolder = graphql(`
    mutation DashboardLayout_PageItem_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const renameFolder = graphql(`
    mutation DashboardLayout_PageItem_RenameFolder_Mutation($input: RenameFolderInput!) {
      renameFolder(input: $input) {
        id
        name
      }
    }
  `);

  const deleteFolder = graphql(`
    mutation DashboardLayout_PageItem_DeleteFolder_Mutation($input: DeleteFolderInput!) {
      deleteFolder(input: $input) {
        id
      }
    }
  `);
</script>

<li
  bind:this={itemEl}
  id={entity.id}
  class={cx(entity.node?.__typename === 'Folder' ? 'dnd-item-folder' : 'dnd-item-page', css({ userSelect: 'none' }))}
  onpointerdown={(e) => onPointerDown(e, entity)}
>
  {#if entity.node?.__typename === 'Folder'}
    <details bind:open>
      <summary
        class={cx(
          'dnd-item-body',
          'group',
          css({
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '8px',
            paddingY: '5px',
            borderRadius: '6px',
            fontSize: '14px',
            cursor: 'pointer',
            listStyleType: 'none',
            fontWeight: 'medium',
            color: 'gray.700',
            _hover: {
              backgroundColor: 'gray.100',
            },
          }),
        )}
        onclick={(e) => {
          if (editing) e.preventDefault();
        }}
      >
        <span
          class={css({
            display: 'none',
            alignItems: 'center',
            flex: 'none',
            width: '16px',
            height: '16px',
            color: 'gray.500',
            _groupHover: { display: 'flex' },
          })}
        >
          {#if open}
            <Icon icon={ChevronUpIcon} size={14} />
          {:else}
            <Icon icon={ChevronDownIcon} size={14} />
          {/if}
        </span>

        <span
          class={css({
            display: 'flex',
            alignItems: 'center',
            flex: 'none',
            width: '16px',
            height: '16px',
            color: 'gray.500',
            _groupHover: { display: 'none' },
          })}
        >
          <Icon icon={FolderIcon} size={14} />
        </span>

        {#if editing}
          <form
            class={css({ fontSize: '14px', flexGrow: '1', minWidth: '0' })}
            onsubmit={async (e) => {
              e.preventDefault();
              if (editing && entity.node?.__typename === 'Folder') {
                await renameFolder({
                  folderId: entity.node.id,
                  name,
                });
                editing = false;
              }
            }}
          >
            <input
              bind:this={inputEl}
              onblur={(e) => e.currentTarget.form?.requestSubmit()}
              onkeydown={(e) => {
                if (e.key === 'Escape' && entity.node?.__typename === 'Folder') {
                  e.preventDefault();
                  name = entity.node.name;
                  editing = false;
                }
              }}
              bind:value={name}
            />
          </form>
        {:else}
          <span class={css({ fontSize: '14px', flexGrow: '1', truncate: true })}>{name}</span>
        {/if}

        <Menu placement="bottom-start">
          {#snippet button({ open })}
            <div
              class={css(
                {
                  display: 'none',
                  borderRadius: '4px',
                  padding: '1px',
                  color: 'gray.400',
                  transition: 'common',
                  _hover: { backgroundColor: 'gray.200' },
                  _groupHover: { display: 'block' },
                },
                open && { display: 'block' },
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
          {#if depth < 2}
            <MenuItem
              onclick={async () => {
                await createFolder({ siteId, name: '새 폴더', parentEntityId: entity.id });
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
                siteId,
                parentEntityId: entity.id,
              });
              open = true;
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
                message: '정말로 이 폴더를 삭제하시겠어요?',
                actionLabel: '삭제',
                actionHandler: async () => {
                  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                  await deleteFolder({ folderId: entity.node!.id });
                },
              });
            }}
          >
            <Icon icon={Trash2Icon} size={12} />
            <span>폴더 삭제</span>
          </MenuItem>
        </Menu>
      </summary>

      {#if entity.children}
        {#if entity.children.length > 0}
          <PageList depth={depth + 1} entities={entity.children} {nodeMap} parent={entity} {siteId} />
        {:else}
          <p
            class={css({
              marginLeft: '16px',
              paddingX: '8px',
              paddingY: '5px',
              fontSize: '14px',
              color: 'gray.400',
            })}
          >
            폴더가 비어있어요
          </p>
        {/if}
      {/if}
    </details>
  {:else}
    <a
      class={cx(
        'dnd-item-body',
        'group',
        css({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '5px',
          borderRadius: '6px',
          fontWeight: 'medium',
          fontSize: '14px',
          color: 'gray.600',
          textDecoration: 'none',
          _hover: {
            backgroundColor: 'gray.100',
          },
        }),
      )}
      draggable="false"
      href="/{entity.slug}"
    >
      <span
        class={css({
          display: 'flex',
          alignItems: 'center',
          flex: 'none',
          width: '16px',
          height: '16px',
          color: 'gray.500',
        })}
      >
        <Icon icon={FileIcon} size={14} />
      </span>
      <span class={css({ fontSize: '14px', lineHeight: '[1.2]', flexGrow: '1', truncate: true })}>
        {entity.node?.title ?? '(제목 없음)'}
      </span>

      <Menu placement="bottom-start">
        {#snippet button({ open })}
          <div
            class={css(
              {
                display: 'none',
                borderRadius: '4px',
                padding: '1px',
                color: 'gray.400',
                transition: 'common',
                _hover: { backgroundColor: 'gray.200' },
                _groupHover: { display: 'block' },
              },
              open && { display: 'block' },
            )}
          >
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        <MenuItem
          onclick={async () => {
            const resp = await duplicatePost({
              // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
              postId: entity.node!.id,
            });
            open = true;
            await goto(`/${resp.entity.slug}`);
          }}
        >
          <Icon icon={CopyIcon} size={12} />
          <span>포스트 복제</span>
        </MenuItem>
        <MenuItem
          onclick={async () => {
            Dialog.confirm({
              title: '포스트 삭제',
              message: '정말로 이 포스트를 삭제하시겠어요?',
              actionLabel: '삭제',
              actionHandler: async () => {
                // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                await deletePost({ postId: entity.node!.id });
              },
            });
          }}
        >
          <Icon icon={Trash2Icon} size={12} />
          <span>포스트 삭제</span>
        </MenuItem>
      </Menu>
    </a>
  {/if}
</li>
