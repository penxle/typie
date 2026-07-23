<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { HorizontalDivider, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { showPasteToast } from './paste-toast';

  let { siteId, lastRootEntityOrder }: { siteId: string; lastRootEntityOrder: string | null } = $props();

  const app = getAppContext();

  const [createDocument] = createMutation(
    graphql(`
      mutation TreeRootMenu_CreateDocument_Mutation($input: CreateDocumentInput!) {
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
            }
          }
        }
      }
    `),
  );

  const [createFolder] = createMutation(
    graphql(`
      mutation TreeRootMenu_CreateFolder_Mutation($input: CreateFolderInput!) {
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
            }
          }
        }
      }
    `),
  );

  const [moveEntities] = createMutation(
    graphql(`
      mutation TreeRootMenu_MoveEntities_Mutation($input: MoveEntitiesInput!) {
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

  const [copyEntities] = createMutation(
    graphql(`
      mutation TreeRootMenu_CopyEntities_Mutation($input: CopyEntitiesInput!) {
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

  const handlePaste = () => {
    const clipboard = app.state.clipboard;
    if (!clipboard) return;

    if (!SubscribeModal.gate('entity_paste')) {
      return;
    }

    const count = clipboard.entityIds.length;

    const promise = (async () => {
      if (clipboard.mode === 'cut') {
        const isCrossSite = clipboard.sourceSiteId !== siteId;

        await moveEntities({
          input: {
            entityIds: clipboard.entityIds,
            parentEntityId: null,
            lowerOrder: lastRootEntityOrder,
            upperOrder: null,
            ...(isCrossSite && { targetSiteId: siteId }),
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
            targetSiteId: siteId,
            parentEntityId: null,
            lowerOrder: lastRootEntityOrder,
            upperOrder: null,
          },
        });
      }
    })();

    showPasteToast(promise, count);
  };
</script>

<MenuItem
  icon={SquarePenIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('tree_root_menu_create_document')) {
      return;
    }

    const resp = await createDocument({ input: { siteId, v2: true } });
    mixpanel.track('create_document', { via: 'tree_root_menu' });
    await goto(`/${resp.createDocument.entity.slug}`);
  }}
>
  새 문서 생성
</MenuItem>

<MenuItem
  icon={FolderPlusIcon}
  onclick={async () => {
    if (!SubscribeModal.gate('tree_root_menu_create_folder')) {
      return;
    }

    const resp = await createFolder({ input: { siteId, name: '새 폴더' } });
    mixpanel.track('create_folder', { via: 'tree_root_menu' });
    app.state.newFolderId = resp.createFolder.id;
  }}
>
  새 폴더 생성
</MenuItem>

{#if app.state.clipboard}
  <HorizontalDivider color="secondary" />

  <MenuItem icon={ClipboardPasteIcon} onclick={handlePaste}>붙여넣기</MenuItem>
{/if}
