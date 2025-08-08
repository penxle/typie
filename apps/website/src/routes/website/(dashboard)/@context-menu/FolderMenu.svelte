<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem, RingSpinner } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { EntityType, EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
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
  import { graphql } from '$graphql';
  import { maxDepth } from '../@tree/utils';

  type Props = {
    folder: {
      id: string;
      name: string;
    };
    entity: {
      id: string;
      url: string;
      depth: number;
      visibility: EntityVisibility;
      site: {
        id: string;
      };
    };
    via: 'tree';
    open: () => void;
    onRename: () => void;
  };

  let { folder, entity, via, onRename, open }: Props = $props();

  const descendants = graphql(`
    query FolderMenu_Descendants_Query($entityId: ID!) @client {
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
    query FolderMenu_Info_Query($folderId: ID!) @client {
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
    mutation FolderMenu_CreatePost_Mutation($input: CreatePostInput!) {
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
    mutation FolderMenu_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const createCanvas = graphql(`
    mutation FolderMenu_CreateCanvas_Mutation($input: CreateCanvasInput!) {
      createCanvas(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deleteFolder = graphql(`
    mutation FolderMenu_DeleteFolder_Mutation($input: DeleteFolderInput!) {
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

  let loadingDescendants = $state(false);
  let loadingInfo = $state(false);

  $effect(() => {
    loadingInfo = true;
    info.load({ folderId: folder.id }).then(() => {
      loadingInfo = false;
    });
  });
</script>

<MenuItem
  icon={PencilIcon}
  onclick={() => {
    if (onRename) {
      onRename();
    }
    mixpanel.track('rename_folder', { via });
  }}
>
  이름 변경
</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem external href={entity.url} icon={ExternalLinkIcon} type="link">사이트에서 열기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={BlendIcon}
  onclick={() => {
    app.state.shareOpen = entity.id;
    mixpanel.track('open_folder_share_modal', { via });
  }}
>
  공유 및 게시
</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={SquarePenIcon}
  onclick={async () => {
    const resp = await createPost({
      siteId: entity.site.id,
      parentEntityId: entity.id,
    });

    mixpanel.track('create_child_post', { via });
    open();
    await goto(`/${resp.entity.slug}`);
  }}
>
  하위 포스트 생성
</MenuItem>

<MenuItem
  icon={LineSquiggleIcon}
  onclick={async () => {
    const resp = await createCanvas({
      siteId: entity.site.id,
      parentEntityId: entity.id,
    });

    mixpanel.track('create_child_canvas', { via });
    open();
    await goto(`/${resp.entity.slug}`);
  }}
>
  하위 캔버스 생성
</MenuItem>

{#if entity.depth < maxDepth - 1}
  <MenuItem
    icon={FolderPlusIcon}
    onclick={async () => {
      const resp = await createFolder({
        siteId: entity.site.id,
        parentEntityId: entity.id,
        name: '새 폴더',
      });

      mixpanel.track('create_child_folder', { via });
      open();
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
    descendants.load({ entityId: entity.id }).then(() => {
      loadingDescendants = false;
    });

    Dialog.confirm({
      title: '폴더 삭제',
      message: `정말 "${folder.name}" 폴더를 삭제하시겠어요?`,
      children: descendantsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteFolder({ folderId: folder.id });
        mixpanel.track('delete_folder', { via });
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
    {#if entity.visibility === EntityVisibility.UNLISTED}
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

{#snippet descendantsView()}
  {#if loadingDescendants || !$descendants}
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
