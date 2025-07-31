<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { graphql } from '$graphql';
  import { HorizontalDivider, Icon, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog, Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { TreeEntity } from './types';

  const app = getAppContext();

  const deleteEntities = graphql(`
    mutation DashboardLayout_EntityTree_MultiEntitiesMenu_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
      deleteEntities(input: $input) {
        id
        site {
          id
          ...DashboardLayout_EntityTree_site
          ...DashboardLayout_PlanUsageWidget_site
        }
      }
    }
  `);

  let folderCount = $state(0);
  let postCount = $state(0);
  let canvasCount = $state(0);

  onMount(async () => {
    const entityIds = new Set(app.state.tree.selectedEntityIds);

    const collect = (entities: TreeEntity[]) => {
      entities.forEach((entity) => {
        if (entity.type === 'Folder') {
          if (entityIds.has(entity.id)) {
            folderCount++;
          }

          collect(entity.children ?? []);
        } else if (entityIds.has(entity.id)) {
          if (entity.type === 'Post') {
            postCount++;
          } else if (entity.type === 'Canvas') {
            canvasCount++;
          }
        }
      });
    };

    collect(app.state.tree.entities);
  });
</script>

<div class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', color: 'text.disabled', fontWeight: 'medium' })}>
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    {#if folderCount > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={14} />
        {folderCount}개
      </div>
    {/if}
    {#if postCount > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={FileIcon} size={14} />
        {postCount}개
      </div>
    {/if}
    {#if canvasCount > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={LineSquiggleIcon} size={14} />
        {canvasCount}개
      </div>
    {/if}
  </div>
</div>

<HorizontalDivider color="secondary" />

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
          const entityIds = [...app.state.tree.selectedEntityIds];

          await deleteEntities({ entityIds });

          mixpanel.track('delete_entities', {
            totalCount: entityIds.length,
            via: 'tree',
          });

          app.state.tree.selectedEntityIds.clear();
          app.state.tree.lastSelectedEntityId = undefined;

          if (app.state.current && entityIds.includes(app.state.current)) {
            app.state.ancestors = [];
            app.state.current = undefined;
          }

          Toast.success(`${entityIds.length}개의 항목이 삭제되었어요`);
        } catch {
          Toast.error('삭제 중 오류가 발생했습니다');
        }
      },
    });
  }}
  variant="danger"
>
  일괄 삭제
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
        folderCount > 0 && `${folderCount}개의 폴더`,
        postCount > 0 && `${postCount}개의 포스트`,
        canvasCount > 0 && `${canvasCount}개의 캔버스`,
      ]
        .filter(Boolean)
        .join(', ')}가 삭제돼요
    </span>
  </div>

  {#if folderCount > 0}
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
      <Icon style={css.raw({ color: 'text.muted' })} icon={TriangleAlertIcon} size={14} />
      <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>폴더를 삭제하면 하위 항목도 함께 삭제됩니다</span>
    </div>
  {/if}
{/snippet}
