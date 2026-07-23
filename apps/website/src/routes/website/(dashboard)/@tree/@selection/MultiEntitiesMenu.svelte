<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import BlendIcon from '~icons/lucide/blend';
  import ClipboardCopyIcon from '~icons/lucide/clipboard-copy';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import InfoIcon from '~icons/lucide/info';
  import ScissorsIcon from '~icons/lucide/scissors';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import EntityIconPicker from '../../@context-menu/EntityIconPicker.svelte';
  import { SubscribeModal } from '../../@subscription/subscribe-modal.svelte';
  import { getTreeContext } from '../state.svelte';
  import type { TreeEntity } from './types';

  const app = getAppContext();
  const tree = getTreeContext();

  const [deleteEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_EntityTree_MultiEntitiesMenu_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
        deleteEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_TrashModal_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }
        }
      }
    `),
  );

  const [updateEntitiesIcon] = createMutation(
    graphql(`
      mutation MultiEntitiesMenu_UpdateEntitiesIcon_Mutation($input: UpdateEntitiesIconInput!) {
        updateEntitiesIcon(input: $input) {
          id
          icon
          iconColor
        }
      }
    `),
  );

  const { folderIds, documentIds } = $derived.by(() => {
    const folderIds: string[] = [];
    const documentIds: string[] = [];

    const entityIds = tree.selectedEntityIds;

    const collect = (entities: TreeEntity[]) => {
      for (const entity of entities) {
        if (entityIds.has(entity.id)) {
          if (entity.type === 'Folder') {
            folderIds.push(entity.id);
          } else if (entity.type === 'Document') {
            documentIds.push(entity.id);
          }
        }

        if (entity.children) {
          collect(entity.children);
        }
      }
    };

    collect(tree.entities);

    return { folderIds, documentIds };
  });

  const { allSameIcon, allSameIconColor } = $derived.by(() => {
    const entityIds = tree.selectedEntityIds;
    let firstIcon: string | undefined;
    let firstIconColor: string | undefined;
    let allSameIcon: string | undefined;
    let allSameIconColor: string | undefined;
    let first = true;

    for (const entityId of entityIds) {
      const entity = tree.entityMap.get(entityId);
      if (!entity) continue;

      if (first) {
        firstIcon = entity.icon;
        firstIconColor = entity.iconColor;
        allSameIcon = firstIcon;
        allSameIconColor = firstIconColor;
        first = false;
      } else {
        if (allSameIcon !== undefined && entity.icon !== firstIcon) {
          allSameIcon = undefined;
        }
        if (allSameIconColor !== undefined && entity.iconColor !== firstIconColor) {
          allSameIconColor = undefined;
        }
      }
    }

    return { allSameIcon, allSameIconColor };
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
    {#if documentIds.length > 0}
      <div class={center({ gap: '2px' })}>
        <Icon style={css.raw({ color: 'text.disabled' })} icon={FileIcon} size={14} />
        {documentIds.length}개
      </div>
    {/if}
  </div>
</div>

<HorizontalDivider color="secondary" />

<EntityIconPicker
  icon={allSameIcon}
  iconColor={allSameIconColor}
  onColorSelect={async (color) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    const entityIds = [...tree.selectedEntityIds];
    await updateEntitiesIcon(
      { input: { entityIds, icon: allSameIcon, iconColor: color } },
      {
        metadata: {
          cache: {
            optimisticResponse: {
              updateEntitiesIcon: entityIds.map((id) => ({
                id,
                icon: allSameIcon ?? tree.entityMap.get(id)?.icon ?? 'file',
                iconColor: color,
              })),
            },
          },
        },
      },
    );
  }}
  onIconSelect={async (name) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    const entityIds = [...tree.selectedEntityIds];
    await updateEntitiesIcon(
      { input: { entityIds, icon: name, iconColor: allSameIconColor } },
      {
        metadata: {
          cache: {
            optimisticResponse: {
              updateEntitiesIcon: entityIds.map((id) => ({
                id,
                icon: name,
                iconColor: allSameIconColor ?? tree.entityMap.get(id)?.iconColor ?? 'gray',
              })),
            },
          },
        },
      },
    );
  }}
/>

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

{#if documentIds.length > 0}
  <MenuItem
    icon={BlendIcon}
    onclick={() => {
      app.state.shareOpen = documentIds;
      mixpanel.track('open_document_share_modal', { via: 'multi_entities_menu', count: documentIds.length });
    }}
  >
    문서 {documentIds.length}개 공유 및 게시
  </MenuItem>
{/if}

<MenuItem
  icon={ClipboardCopyIcon}
  onclick={() => {
    const currentSiteId = app.preference.current.currentSiteId;
    if (!currentSiteId) return;

    app.state.clipboard = {
      mode: 'copy',
      entityIds: [...tree.selectedEntityIds],
      sourceSiteId: currentSiteId,
    };
  }}
>
  복사
</MenuItem>

<MenuItem
  icon={ScissorsIcon}
  onclick={() => {
    const currentSiteId = app.preference.current.currentSiteId;
    if (!currentSiteId) return;

    app.state.clipboard = {
      mode: 'cut',
      entityIds: [...tree.selectedEntityIds],
      sourceSiteId: currentSiteId,
    };
  }}
>
  잘라내기
</MenuItem>

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
          const entityIds = [...tree.selectedEntityIds];

          await deleteEntities({ input: { entityIds } });

          if (app.preference.current.currentSiteId) {
            cache.invalidate({ __typename: 'Site', id: app.preference.current.currentSiteId, $field: 'deletedEntities' });
          }

          mixpanel.track('delete_entities', {
            totalCount: entityIds.length,
            via: 'tree',
          });

          tree.selectedEntityIds.clear();
          tree.lastSelectedEntityId = undefined;

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
      {[folderIds.length > 0 && `${folderIds.length}개의 폴더`, documentIds.length > 0 && `${documentIds.length}개의 문서`]
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
