<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { EntityType, EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import InfoIcon from '~icons/lucide/info';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import PencilIcon from '~icons/lucide/pencil-line';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Menu, MenuItem, RingSpinner } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog } from '$lib/notification';
  import { comma } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import Entity from './Entity.svelte';
  import { getTreeContext } from './state.svelte';
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

  const info = graphql(`
    query DashboardLayout_EntityTree_Folder_Info_Query($folderId: ID!) @client {
      folder(id: $folderId) {
        id
        characterCount
        folderCount
        postCount
        canvasCount
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

  const createCanvas = graphql(`
    mutation DashboardLayout_EntityTree_Folder_CreateCanvas_Mutation($input: CreateCanvasInput!) {
      createCanvas(input: $input) {
        id

        entity {
          id
          slug
        }
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

        entity {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_Trash_site
            ...DashboardLayout_PlanUsageWidget_site
          }
        }
      }
    }
  `);

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.ancestors.includes($folder.entity.id));
  const selected = $derived(treeState.selectedEntityIds.has($folder.entity.id));

  let detailsEl = $state<HTMLDetailsElement>();
  let inputEl = $state<HTMLInputElement>();

  let open = $state(false);
  let editing = $state(false);
  let loadingDescendants = $state(false);
  let loadingInfo = $state(false);

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

  $effect(() => {
    if (app.state.newFolderId === $folder.id) {
      editing = true;
      app.state.newFolderId = undefined;

      if (detailsEl) {
        const rect = detailsEl.getBoundingClientRect();
        const isInViewport = rect.top >= 0 && rect.bottom <= window.innerHeight;

        if (!isInViewport) {
          detailsEl.scrollIntoView({ behavior: 'instant', block: 'nearest' });
        }
      }
    }
  });
</script>

<details
  bind:this={detailsEl}
  data-id={$folder.entity.id}
  data-order={$folder.entity.order}
  data-path-depth={$folder.entity.depth}
  data-type="folder"
  bind:open
>
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
        selected && {
          backgroundColor: 'accent.brand.subtle',
          _supportHover: { backgroundColor: 'accent.brand.subtle' },
          '&:has([aria-pressed="true"])': { backgroundColor: 'accent.brand.subtle' },
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
    <EntitySelectionIndicator entityId={$folder.entity.id} visibility={$folder.entity.visibility} />

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

      <Menu
        onopen={() => {
          loadingInfo = true;
          info.load({ folderId: $folder.id }).then(() => {
            loadingInfo = false;
          });
        }}
        placement="bottom-start"
      >
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

        {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has($folder.entity.id)}
          <MultiEntitiesMenu />
        {:else}
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
            공유 및 게시
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

          <MenuItem
            icon={LineSquiggleIcon}
            onclick={async () => {
              const resp = await createCanvas({
                siteId: $folder.entity.site.id,
                parentEntityId: $folder.entity.id,
              });

              mixpanel.track('create_child_canvas');

              await goto(`/${resp.entity.slug}`);
            }}
          >
            하위 캔버스 생성
          </MenuItem>

          {#if $folder.entity.depth < maxDepth - 1}
            <MenuItem
              icon={FolderPlusIcon}
              onclick={async () => {
                const resp = await createFolder({
                  siteId: $folder.entity.site.id,
                  parentEntityId: $folder.entity.id,
                  name: '새 폴더',
                });

                mixpanel.track('create_child_folder');

                open = true;
                app.state.newFolderId = resp.id;
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
                  if (treeState.selectedEntityIds.has($folder.entity.id)) {
                    treeState.selectedEntityIds.delete($folder.entity.id);
                  }
                  if (treeState.lastSelectedEntityId === $folder.entity.id) {
                    treeState.lastSelectedEntityId = undefined;
                  }
                },
              });
            }}
            variant="danger"
          >
            삭제
          </MenuItem>

          <HorizontalDivider color="secondary" />

          <div class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', color: 'text.disabled', userSelect: 'none' })}>
            <div class={css({ fontWeight: 'medium' })}>
              {#if $folder.entity.visibility === EntityVisibility.UNLISTED}
                <span class={css({ color: 'accent.brand.default' })}>링크 조회 가능 폴더</span>
              {:else}
                <span>비공개 폴더</span>
              {/if}
            </div>

            {#if loadingInfo}
              <span class={flex({ alignItems: 'center', gap: '4px' })}>
                <RingSpinner style={css.raw({ size: '12px' })} />
                불러오는 중...
              </span>
            {:else if $info}
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                {#if $info.folder.folderCount > 0}
                  <div class={center({ gap: '2px' })}>
                    <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={14} />
                    {$info.folder.folderCount}개
                  </div>
                {/if}
                {#if $info.folder.postCount > 0}
                  <div class={center({ gap: '2px' })}>
                    <Icon style={css.raw({ color: 'text.disabled' })} icon={FileIcon} size={14} />
                    {$info.folder.postCount}개
                  </div>
                {/if}
                {#if $info.folder.canvasCount > 0}
                  <div class={center({ gap: '2px' })}>
                    <Icon style={css.raw({ color: 'text.disabled' })} icon={LineSquiggleIcon} size={14} />
                    {$info.folder.canvasCount}개
                  </div>
                {/if}
              </div>

              <span>총 {comma($info.folder.characterCount)}자</span>
            {/if}
          </div>
        {/if}
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
      <span class={css({ fontSize: '13px', color: 'text.faint' })}>함께 삭제될 항목 계산중...</span>
    </div>
  {:else}
    {@const folders = $descendants.entity.descendants.filter((d) => d.type === EntityType.FOLDER).length}
    {@const posts = $descendants.entity.descendants.filter((d) => d.type === EntityType.POST).length}
    {@const canvases = $descendants.entity.descendants.filter((d) => d.type === EntityType.CANVAS).length}

    {#if folders > 0 || posts > 0 || canvases > 0}
      {@const items = [
        folders > 0 && `${folders}개의 하위 폴더`,
        posts > 0 && `${posts}개의 하위 포스트`,
        canvases > 0 && `${canvases}개의 하위 캔버스`,
      ].filter(Boolean)}
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
          {items.join('와 ')}가 함께 삭제돼요
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
      <Icon style={css.raw({ color: 'text.muted' })} icon={InfoIcon} size={14} />
      <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>삭제 후 30일 동안 휴지통에 보관돼요</span>
    </div>
  {/if}
{/snippet}
