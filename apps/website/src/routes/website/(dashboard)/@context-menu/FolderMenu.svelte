<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { EntityType, EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem, RingSpinner } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { getContext, tick } from 'svelte';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import ClipboardCopyIcon from '~icons/lucide/clipboard-copy';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import CopyIcon from '~icons/lucide/copy';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import GlobeIcon from '~icons/lucide/globe';
  import InfoIcon from '~icons/lucide/info';
  import PencilIcon from '~icons/lucide/pencil-line';
  import ScissorsIcon from '~icons/lucide/scissors';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import TrashIcon from '~icons/lucide/trash';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { goto } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPaneGroup } from '../[slug]/@pane/context.svelte';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { maxDepth } from '../@tree/utils';
  import EntityIconPicker from './EntityIconPicker.svelte';
  import { showPasteToast } from './paste-toast';

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
      icon: string;
      iconColor: string;
      lastChild?: {
        id: string;
        order: string;
      } | null;
      site: {
        id: string;
      };
    };
    via: 'tree';
    open: () => void;
    onRename: () => void;
  };

  let { folder, entity, via, onRename, open }: Props = $props();

  let deleteOpen = $state(false);

  const descendants = createQuery(
    graphql(`
      query FolderMenu_Descendants_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          descendants {
            id
            type
          }
        }
      }
    `),
    () => ({ entityId: entity.id }),
    () => ({ skip: !deleteOpen }),
  );

  const info = createQuery(
    graphql(`
      query FolderMenu_Info_Query($folderId: ID!) {
        folder(id: $folderId) {
          id
          characterCount
          folderCount
          documentCount
        }
      }
    `),
    () => ({ folderId: folder.id }),
  );

  const [createDocument] = createMutation(
    graphql(`
      mutation FolderMenu_CreateDocument_Mutation($input: CreateDocumentInput!) {
        createDocument(input: $input) {
          id

          entity {
            id
            slug

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
      }
    `),
  );

  const [createFolder] = createMutation(
    graphql(`
      mutation FolderMenu_CreateFolder_Mutation($input: CreateFolderInput!) {
        createFolder(input: $input) {
          id

          entity {
            id

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
      }
    `),
  );

  const [deleteFolder] = createMutation(
    graphql(`
      mutation FolderMenu_DeleteFolder_Mutation($input: DeleteFolderInput!) {
        deleteFolder(input: $input) {
          id

          entity {
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
      }
    `),
  );

  const [copyEntities] = createMutation(
    graphql(`
      mutation FolderMenu_CopyEntities_Mutation($input: CopyEntitiesInput!) {
        copyEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
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

  const [moveEntities] = createMutation(
    graphql(`
      mutation FolderMenu_MoveEntities_Mutation($input: MoveEntitiesInput!) {
        moveEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
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

          children {
            id

            node {
              __typename
            }

            ...DashboardLayout_EntityTree_Entity_entity
          }

          ancestors {
            id

            node {
              __typename

              ... on Folder {
                id
                name
              }
            }
          }

          parent {
            id
          }
        }
      }
    `),
  );

  const [updateEntityIcon] = createMutation(
    graphql(`
      mutation FolderMenu_UpdateEntityIcon_Mutation($input: UpdateEntityIconInput!) {
        updateEntityIcon(input: $input) {
          id
          icon
          iconColor
        }
      }
    `),
  );

  const close = getContext<undefined | (() => void)>('close');
  const app = getAppContext();
  const paneGroup = getPaneGroup();
</script>

<MenuItem
  icon={PencilIcon}
  onclick={() => {
    mixpanel.track('rename_folder_try', { via });
    if (onRename) {
      onRename();
    }
  }}
>
  이름 변경
</MenuItem>

<EntityIconPicker
  icon={entity.icon}
  iconColor={entity.iconColor}
  onColorSelect={async (color) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    await updateEntityIcon(
      { input: { entityId: entity.id, icon: entity.icon, iconColor: color } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entity.id, icon: entity.icon, iconColor: color } } } } },
    );
  }}
  onIconSelect={async (name) => {
    if (!SubscribeModal.gate('entity_update_icon')) {
      return;
    }

    await updateEntityIcon(
      { input: { entityId: entity.id, icon: name, iconColor: entity.iconColor } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entity.id, icon: name, iconColor: entity.iconColor } } } } },
    );
  }}
/>

<HorizontalDivider color="secondary" />

<MenuItem external href={entity.url} icon={GlobeIcon} type="link">스페이스에서 열기</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem
  icon={BlendIcon}
  onclick={() => {
    app.state.shareOpen = [entity.id];
    mixpanel.track('open_folder_share_modal', { via });
  }}
>
  공유 및 게시
</MenuItem>

<MenuItem
  icon={ClipboardCopyIcon}
  onclick={() => {
    app.state.clipboard = {
      mode: 'copy',
      entityIds: [entity.id],
      sourceSiteId: entity.site.id,
    };
  }}
>
  복사
</MenuItem>

<MenuItem
  icon={ScissorsIcon}
  onclick={() => {
    app.state.clipboard = {
      mode: 'cut',
      entityIds: [entity.id],
      sourceSiteId: entity.site.id,
    };
  }}
>
  잘라내기
</MenuItem>

{#if app.state.clipboard}
  <MenuItem
    icon={ClipboardPasteIcon}
    onclick={() => {
      const clipboard = app.state.clipboard;
      if (!clipboard) return;

      if (!SubscribeModal.gate('entity_paste')) {
        return;
      }

      const lowerOrder = entity.lastChild?.order ?? null;
      const count = clipboard.entityIds.length;

      const promise = (async () => {
        if (clipboard.mode === 'cut') {
          const isCrossSite = clipboard.sourceSiteId !== entity.site.id;
          await moveEntities({
            input: {
              entityIds: clipboard.entityIds,
              parentEntityId: entity.id,
              lowerOrder,
              upperOrder: null,
              ...(isCrossSite && { targetSiteId: entity.site.id }),
            },
          });
          if (isCrossSite) {
            cache.invalidate({ __typename: 'Site', id: clipboard.sourceSiteId, $field: 'entities' });
          }
          app.state.clipboard = undefined;
        } else {
          await copyEntities({
            input: {
              entityIds: clipboard.entityIds,
              targetSiteId: entity.site.id,
              parentEntityId: entity.id,
              lowerOrder,
              upperOrder: null,
            },
          });
        }
      })();

      showPasteToast(promise, count);
    }}
  >
    여기에 붙여넣기
  </MenuItem>
{/if}

<HorizontalDivider color="secondary" />

<MenuItem
  icon={SquarePenIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('folder_menu_create_document')) {
      return;
    }

    const resp = await createDocument({
      input: {
        siteId: entity.site.id,
        parentEntityId: entity.id,
        v2: true,
      },
    });

    mixpanel.track('create_child_document', { via });
    open();
    await goto(`/${resp.createDocument.entity.slug}`);
  }}
>
  하위 문서 생성
</MenuItem>

{#if entity.depth < maxDepth - 1}
  <MenuItem
    icon={FolderPlusIcon}
    onclick={async () => {
      if (!SubscribeModal.gate('folder_menu_create_folder')) {
        return;
      }

      const resp = await createFolder({
        input: {
          siteId: entity.site.id,
          parentEntityId: entity.id,
          name: '새 폴더',
        },
      });

      mixpanel.track('create_child_folder', { via });
      open();

      // NOTE: 메뉴 닫힘/포커스 복귀 사이클 이후 실행되도록 다음 tick으로 미룬다.
      await tick();
      app.state.newFolderId = resp.createFolder.id;
    }}
  >
    하위 폴더 생성
  </MenuItem>
{/if}

<HorizontalDivider color="secondary" />

<MenuItem
  icon={TrashIcon}
  noCloseOnClick
  onclick={() => {
    deleteOpen = true;

    Dialog.confirm({
      title: '폴더 삭제',
      message: `정말 "${folder.name}" 폴더를 삭제하시겠어요?`,
      children: descendantsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteFolder({ input: { folderId: folder.id } });
        mixpanel.track('delete_folder', { via });

        if (!app.state.ancestors.includes(entity.id)) return;

        const focusedPaneId = paneGroup.state.current.focusedPaneId;
        if (!focusedPaneId) return;

        if (paneGroup.panes.length > 1) {
          paneGroup.removePane(focusedPaneId);
        } else {
          paneGroup.replacePane(focusedPaneId, { kind: 'home' });
        }
      },
      onclose: () => {
        deleteOpen = false;
        close?.();
      },
    });
  }}
  variant="danger"
>
  삭제
</MenuItem>

<HorizontalDivider color="secondary" />

<div
  class={flex({
    flexDirection: 'column',
    gap: '4px',
    paddingX: '10px',
    paddingY: '4px',
    fontSize: '12px',
    color: 'text.disabled',
    userSelect: 'none',
  })}
>
  <div class={css({ fontWeight: 'medium' })}>
    {#if entity.visibility === EntityVisibility.PUBLIC}
      <span class={css({ color: 'accent.success.default' })}>공개 폴더</span>
    {:else if entity.visibility === EntityVisibility.UNLISTED}
      <span class={css({ color: 'accent.brand.default' })}>링크 조회 가능 폴더</span>
    {:else}
      <span>비공개 폴더</span>
    {/if}
  </div>

  {#if info.loading}
    <span class={flex({ alignItems: 'center', gap: '4px' })}>
      <RingSpinner style={css.raw({ size: '12px' })} />
      불러오는 중...
    </span>
  {:else if info.data}
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      {#if info.data.folder.folderCount > 0}
        <div class={center({ gap: '2px' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={14} />
          {info.data.folder.folderCount}개
        </div>
      {/if}
      {#if info.data.folder.documentCount > 0}
        <div class={center({ gap: '2px' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={FileIcon} size={14} />
          {info.data.folder.documentCount}개
        </div>
      {/if}
    </div>

    <span>총 {comma(info.data.folder.characterCount)}자</span>
  {/if}

  <button
    class={flex({
      alignItems: 'center',
      gap: '2px',
      width: 'fit',
      cursor: 'pointer',
      fontSize: '11px',
      color: 'text.disabled',
      transition: 'common',
      _hover: { color: 'text.muted' },
    })}
    onclick={async () => {
      await navigator.clipboard.writeText(folder.id);
      Toast.success('폴더 ID가 복사되었어요');
    }}
    type="button"
  >
    <Icon icon={CopyIcon} size={12} />
    폴더 ID 복사
  </button>
</div>

{#snippet descendantsView()}
  {#if descendants.loading}
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
  {:else if descendants.data}
    {@const folders = descendants.data.entity.descendants.filter((d) => d.type === EntityType.FOLDER).length}
    {@const documents = descendants.data.entity.descendants.filter((d) => d.type === EntityType.DOCUMENT).length}

    {#if folders > 0 || documents > 0}
      {@const items = [folders > 0 && `${folders}개의 하위 폴더`, documents > 0 && `${documents}개의 하위 문서`].filter(Boolean)}
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
{/snippet}
