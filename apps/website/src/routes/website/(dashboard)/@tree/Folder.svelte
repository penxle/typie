<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { EntityType, EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ExternalLinkIcon from '~icons/lucide/external-link';
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
          url

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
      setTimeout(() => {
        inputEl?.select();
      }, 0);
    }
  });

  $effect.pre(() => {
    if (active) {
      open = true;
    }
  });
</script>

<details data-id={$folder.entity.id} data-order={$folder.entity.order} data-path-depth={$folder.entity.depth} data-type="folder" bind:open>
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
          _supportHover: { backgroundColor: 'surface.muted' },
          '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
        },
        $folder.entity.depth > 0 && {
          borderLeftWidth: '1px',
          borderLeftRadius: '0',
          marginLeft: '-1px',
          paddingLeft: '14px',
          _supportHover: { borderColor: 'border.strong' },
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
        { flex: 'none', borderRadius: 'full', backgroundColor: 'interactive.hover', size: '4px' },
        $folder.entity.visibility === EntityVisibility.UNLISTED && { backgroundColor: 'accent.brand.default' },
      )}
    ></div>

    <Icon style={css.raw({ color: 'text.faint' })} icon={open ? ChevronDownIcon : ChevronRightIcon} size={14} />

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

          mixpanel.track('rename_folder');

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
            color: 'text.muted',
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
          color: 'text.muted',
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
              color: 'text.disabled',
              opacity: '0',
              transition: 'common',
              _hover: { backgroundColor: 'interactive.hover' },
              _groupHover: { opacity: '100' },
              _pressed: { backgroundColor: 'interactive.hover', opacity: '100' },
            })}
            aria-pressed={open}
          >
            <Icon icon={EllipsisIcon} size={14} />
          </div>
        {/snippet}

        <MenuItem icon={PencilIcon} onclick={() => (editing = true)}>이름 변경</MenuItem>

        <HorizontalDivider color="secondary" />

        <MenuItem external href={$folder.entity.url} icon={ExternalLinkIcon} type="link">사이트에서 열기</MenuItem>

        <HorizontalDivider color="secondary" />

        <MenuItem
          icon={BlendIcon}
          onclick={() => {
            app.state.shareOpen = $folder.entity.id;
            mixpanel.track('open_folder_share_modal');
          }}
        >
          공유
        </MenuItem>

        <HorizontalDivider color="secondary" />

        <MenuItem
          icon={SquarePenIcon}
          onclick={async () => {
            const resp = await createPost({
              siteId: $folder.entity.site.id,
              parentEntityId: $folder.entity.id,
            });

            mixpanel.track('create_child_post');

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

              mixpanel.track('create_child_folder');

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
              message: `정말 "${$folder.name}" 폴더를 삭제하시겠어요?`,
              children: descendantsView,
              action: 'danger',
              actionLabel: '삭제',
              actionHandler: async () => {
                await deleteFolder({ folderId: $folder.id });
                mixpanel.track('delete_folder');
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
      <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
        폴더가 비어있어요
      </div>
    {/each}
  </div>
</details>

{#snippet descendantsView()}
  {#if !$descendants || loadingDescendants}
    <div
      class={flex({
        alignItems: 'center',
        gap: '6px',
        borderRadius: '8px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.subtle',
      })}
    >
      <RingSpinner style={css.raw({ size: '13px', color: 'text.faint' })} />
      <span class={css({ fontSize: '13px', color: 'text.faint' })}>함께 삭제될 폴더와 포스트 계산중...</span>
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
          backgroundColor: 'accent.danger.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.danger' })} icon={TriangleAlertIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.danger' })}>
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
          backgroundColor: 'accent.success.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.success' })} icon={CheckIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.success' })}>비어있는 폴더에요</span>
      </div>
    {/if}
  {/if}
{/snippet}
