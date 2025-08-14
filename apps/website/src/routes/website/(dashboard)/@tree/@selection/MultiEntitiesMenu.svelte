<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import BlendIcon from '~icons/lucide/blend';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import InfoIcon from '~icons/lucide/info';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { graphql } from '$graphql';
  import type { TreeState } from '../state.svelte';
  import type { TreeEntity } from './types';

  type Props = {
    treeState: TreeState;
  };

  let { treeState }: Props = $props();

  const app = getAppContext();

  const deleteEntities = graphql(`
    mutation DashboardLayout_EntityTree_MultiEntitiesMenu_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
      deleteEntities(input: $input) {
        id
        site {
          id
          ...DashboardLayout_EntityTree_site
          ...DashboardLayout_Trash_site
          ...DashboardLayout_PlanUsageWidget_site
        }
      }
    }
  `);

  let folderIds = $state<string[]>([]);
  let postIds = $state<string[]>([]);
  let canvasIds = $state<string[]>([]);

  onMount(async () => {
    const entityIds = new Set(treeState.selectedEntityIds);

    const collect = (entities: TreeEntity[]) => {
      entities.forEach((entity) => {
        if (entity.type === 'Folder') {
          if (entityIds.has(entity.id)) {
            folderIds.push(entity.id);
          }

          collect(entity.children ?? []);
        } else if (entityIds.has(entity.id)) {
          if (entity.type === 'Post') {
            postIds.push(entity.id);
          } else if (entity.type === 'Canvas') {
            canvasIds.push(entity.id);
          }
        }
      });
    };

    collect(treeState.entities);
  });
</script>

<div class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', color: 'text.disabled', fontWeight: 'medium' })}>
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    {#if folderIds.length > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={14} />
        {folderIds.length}개
      </div>
    {/if}
    {#if postIds.length > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={FileIcon} size={14} />
        {postIds.length}개
      </div>
    {/if}
    {#if canvasIds.length > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={LineSquiggleIcon} size={14} />
        {canvasIds.length}개
      </div>
    {/if}
  </div>
</div>

<HorizontalDivider color="secondary" />

{#if folderIds.length > 0}
  <MenuItem
    icon={BlendIcon}
    onclick={() => {
      app.state.shareOpen = folderIds;
      mixpanel.track('open_folder_share_modal', { via: 'multi_entities_menu', count: folderIds.length });
    }}
  >
    폴더 {folderIds.length}개 공유 및 게시
  </MenuItem>
{/if}

{#if postIds.length > 0}
  <MenuItem
    icon={BlendIcon}
    onclick={() => {
      app.state.shareOpen = postIds;
      mixpanel.track('open_post_share_modal', { via: 'multi_entities_menu', count: postIds.length });
    }}
  >
    포스트 {postIds.length}개 공유 및 게시
  </MenuItem>
{/if}

<MenuItem
  icon={TrashIcon}
  onclick={async () => {
    Dialog.confirm({
      title: '선택한 항목 삭제',
      message: `정말 선택한 항목을 삭제하시겠어요?`,
      children: deleteDetailsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        try {
          const entityIds = [...treeState.selectedEntityIds];

          await deleteEntities({ entityIds });

          mixpanel.track('delete_entities', {
            totalCount: entityIds.length,
            via: 'tree',
          });

          treeState.selectedEntityIds.clear();
          treeState.lastSelectedEntityId = undefined;

          Toast.success(`${entityIds.length}개의 항목이 삭제되었어요`);
        } catch {
          Toast.error('삭제 중 오류가 발생했습니다');
        }
      },
    });
  }}
  variant="danger"
>
  삭제
</MenuItem>

{#snippet deleteDetailsView()}
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
      {[
        folderIds.length > 0 && `${folderIds.length}개의 폴더`,
        postIds.length > 0 && `${postIds.length}개의 포스트`,
        canvasIds.length > 0 && `${canvasIds.length}개의 캔버스`,
      ]
        .filter(Boolean)
        .join(', ')}가 삭제돼요
    </span>
  </div>

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
{/snippet}
