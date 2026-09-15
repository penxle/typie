<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { untrack } from 'svelte';
  import { graphql } from '$mearie';
  import DocumentShare from './DocumentShare.svelte';
  import DocumentsShare from './DocumentsShare.svelte';
  import Folder from './Folder.svelte';

  const app = getAppContext();

  const entitiesQuery = createQuery(
    graphql(`
      query DashboardLayout_ShareModal_Query($entityIds: [ID!]!) {
        entities(entityIds: $entityIds) {
          id

          node {
            __typename

            ... on Folder {
              id

              ...DashboardLayout_Share_Folder_folder
            }

            ... on Document {
              id

              publication {
                id
                state
              }

              ...DashboardLayout_Share_DocumentsShare_document
            }
          }
        }
      }
    `),
    () => ({ entityIds: app.state.shareOpen }),
    () => ({ skip: app.state.shareOpen.length === 0 }),
  );

  const entities = $derived(entitiesQuery.data?.entities ?? []);
  const documentNodes = $derived(entities.map((entity) => entity.node).filter((node) => node.__typename === 'Document'));
  const folderNodes = $derived(entities.map((entity) => entity.node).filter((node) => node.__typename === 'Folder'));
  const allDocuments = $derived(entities.length > 0 && documentNodes.length === entities.length);
  const allFolders = $derived(entities.length > 0 && folderNodes.length === entities.length);

  const singleDocumentEntityId = $derived(
    app.state.shareOpen.length === 1 && entities.length === 1 && documentNodes.length === 1 ? entities[0].id : null,
  );

  const documentQuery = createQuery(
    graphql(`
      query DashboardLayout_ShareModal_Document_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          node {
            __typename

            ... on Document {
              id

              ...DashboardLayout_Share_DocumentShare_document
            }
          }
        }
      }
    `),
    () => ({ entityId: singleDocumentEntityId ?? '' }),
    () => ({ skip: singleDocumentEntityId === null }),
  );

  const singleDocument = $derived(
    singleDocumentEntityId !== null && documentQuery.data?.entity.node.__typename === 'Document' ? documentQuery.data.entity.node : null,
  );

  const loaded = $derived(
    app.state.shareOpen.length > 0 &&
      !!entitiesQuery.data &&
      !entitiesQuery.loading &&
      (singleDocumentEntityId === null || (!!singleDocument && !documentQuery.loading)),
  );

  let step = $state<'visibility' | 'publish'>('visibility');
  let steppedFor = $state<string | null>(null);

  $effect(() => {
    if (app.state.shareOpen.length === 0) {
      untrack(() => (steppedFor = null));
      return;
    }

    if (!allDocuments) return;

    const key = documentNodes.map((node) => node.id).join(',');
    const anyPublishing = documentNodes.some((node) => node.publication && node.publication.state !== 'UNPUBLISHED');

    untrack(() => {
      if (steppedFor === key) return;
      steppedFor = key;
      step = anyPublishing ? 'publish' : 'visibility';
    });
  });

  const modalStyle = $derived(
    allDocuments && step === 'publish'
      ? css.raw({ maxWidth: '880px', height: 'full', maxHeight: '680px', padding: '0', overflow: 'hidden' })
      : css.raw({ maxWidth: '480px', padding: '0' }),
  );

  const close = () => {
    app.state.shareOpen = [];
  };
</script>

<Modal style={modalStyle} loading={!loaded} onclose={close} open={app.state.shareOpen.length > 0}>
  {#if loaded}
    {#if singleDocument}
      <DocumentShare document$key={singleDocument} onclose={close} bind:step />
    {:else if allDocuments}
      <DocumentsShare documents$key={documentNodes} onclose={close} bind:step />
    {:else if allFolders}
      <Folder folders$key={folderNodes} onclose={close} />
    {/if}
  {/if}
</Modal>
