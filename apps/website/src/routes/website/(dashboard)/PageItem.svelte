<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PencilIcon from '~icons/lucide/pencil';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { Icon, Menu, MenuItem } from '$lib/components';
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

  const entityQuery = graphql(`
    query DashboardLayout_PageItem_Query($id: ID!) @manual {
      entity(id: $id) {
        id
        slug

        children {
          __typename
          id
          slug
          order

          node {
            ... on Folder {
              __typename
              id
              name
            }

            ... on Post {
              __typename
              id
              title
            }
          }

          children {
            __typename
            id
            slug
            order
          }
        }
      }
    }
  `);

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

  const loadEntity = async () => {
    await entityQuery.refetch({ id: entity.id });
  };
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
            paddingY: '6px',
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
        <span class={css({ display: 'flex', alignItems: 'center', flex: 'none', width: '16px', height: '16px', color: 'gray.500' })}>
          {#if open}
            <Icon icon={ChevronUpIcon} size={14} />
          {:else}
            <Icon icon={ChevronDownIcon} size={14} />
          {/if}
        </span>
        <span class={css({ display: 'flex', alignItems: 'center', flex: 'none', width: '16px', height: '16px', color: 'gray.500' })}>
          <Icon icon={FolderIcon} size={14} />
        </span>

        {#if editing}
          <form
            class={css({ fontSize: '14px', flexGrow: '1', minWidth: '0' })}
            onsubmit={async (e) => {
              e.preventDefault();
              if (editing && entity.node?.__typename === 'Folder') {
                await renameFolder({
                  id: entity.node.id,
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
          <MenuItem
            onclick={async () => {
              await createFolder({ siteId, name: '새 폴더', parentEntityId: entity.id });
              await loadEntity();
              open = true;
            }}
          >
            <Icon icon={FolderPlusIcon} size={12} />
            <span>하위 폴더 생성</span>
          </MenuItem>
          <MenuItem
            onclick={async () => {
              const resp = await createPost({
                siteId,
                parentEntityId: entity.id,
              });
              await loadEntity();
              open = true;
              await goto(`/${resp.entity.slug}`);
            }}
          >
            <Icon icon={FilePlusIcon} size={12} />
            <span>하위 포스트 생성</span>
          </MenuItem>
        </Menu>
      </summary>

      {#if open}
        <PageList depth={depth + 1} {nodeMap} parent={entity} {siteId} />
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
          paddingY: '6px',
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
          marginLeft: '8px',
          color: 'gray.500',
        })}
      >
        <Icon icon={FileIcon} size={14} />
      </span>
      <span class={css({ fontSize: '14px', lineHeight: '[1.2]', flexGrow: '1', truncate: true })}>
        {entity.node?.title ?? '(제목 없음)'}
      </span>
    </a>
  {/if}
</li>
