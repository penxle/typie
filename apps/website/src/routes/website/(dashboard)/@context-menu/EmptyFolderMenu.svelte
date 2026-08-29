<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { HorizontalDivider, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import MinusIcon from '~icons/lucide/minus';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { createEntityTreeRevealRequest, entityTreeRevealState } from '../@tree/entity-reveal.svelte';
  import { maxDepth } from '../@tree/utils';
  import { showPasteToast } from './paste-toast';

  type Props = {
    entity: {
      id: string;
      depth: number;
      site: {
        id: string;
      };
    };
  };

  let { entity }: Props = $props();

  const app = getAppContext();

  const [createDocument] = createMutation(
    graphql(`
      mutation EmptyFolderMenu_CreateDocument_Mutation($input: CreateDocumentInput!) {
        createDocument(input: $input) {
          id

          entity {
            id
            slug

            container {
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
      mutation EmptyFolderMenu_CreateFolder_Mutation($input: CreateFolderInput!) {
        createFolder(input: $input) {
          id

          entity {
            id

            container {
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

  const [createDivider] = createMutation(
    graphql(`
      mutation EmptyFolderMenu_CreateDivider_Mutation($input: CreateDividerInput!) {
        createDivider(input: $input) {
          id

          entity {
            id

            container {
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
      mutation EmptyFolderMenu_CopyEntities_Mutation($input: CopyEntitiesInput!) {
        copyEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
          }

          container {
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
      mutation EmptyFolderMenu_MoveEntities_Mutation($input: MoveEntitiesInput!) {
        moveEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
          }

          container {
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

          parent {
            id
          }
        }
      }
    `),
  );
</script>

{#if entity.depth < maxDepth - 1}
  <MenuItem
    icon={FolderPlusIcon}
    onclick={async () => {
      if (!SubscribeModal.gate('empty_folder_menu_create_folder')) {
        return;
      }

      const resp = await createFolder({
        input: {
          siteId: entity.site.id,
          parentEntityId: entity.id,
          name: '새 폴더',
        },
      });

      mixpanel.track('create_child_folder', { via: 'empty_folder_menu' });

      // NOTE: 메뉴 닫힘/포커스 복귀 사이클 이후 실행되도록 다음 tick으로 미룬다.
      await tick();
      entityTreeRevealState.set(createEntityTreeRevealRequest(resp.createFolder.entity.id, [entity.id], true));
    }}
  >
    하위 폴더 생성
  </MenuItem>
{/if}

<MenuItem
  icon={SquarePenIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('empty_folder_menu_create_document')) {
      return;
    }

    const resp = await createDocument({
      input: {
        siteId: entity.site.id,
        parentEntityId: entity.id,
        v2: true,
      },
    });

    mixpanel.track('create_child_document', { via: 'empty_folder_menu' });
    entityTreeRevealState.set(createEntityTreeRevealRequest(resp.createDocument.entity.id, [entity.id], false));
    await goto(`/${resp.createDocument.entity.slug}`);
  }}
>
  하위 문서 생성
</MenuItem>

<MenuItem
  icon={MinusIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('empty_folder_menu_create_divider')) {
      return;
    }

    const resp = await createDivider({
      input: {
        siteId: entity.site.id,
        parentEntityId: entity.id,
      },
    });

    mixpanel.track('create_child_divider', { via: 'empty_folder_menu' });
    entityTreeRevealState.set(createEntityTreeRevealRequest(resp.createDivider.entity.id, [entity.id], false));
  }}
>
  하위 구분선 삽입
</MenuItem>

{#if app.state.clipboard}
  <HorizontalDivider color="secondary" />

  <MenuItem
    icon={ClipboardPasteIcon}
    onclick={() => {
      const clipboard = app.state.clipboard;
      if (!clipboard) return;

      if (!SubscribeModal.gate('entity_paste')) {
        return;
      }

      const count = clipboard.entityIds.length;

      const promise = (async () => {
        if (clipboard.mode === 'cut') {
          const isCrossSite = clipboard.sourceSiteId !== entity.site.id;
          await moveEntities({
            input: {
              entityIds: clipboard.entityIds,
              parentEntityId: entity.id,
              lowerOrder: null,
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
              lowerOrder: null,
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
