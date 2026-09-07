<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { untrack } from 'svelte';
  import { graphql } from '$mearie';
  import Document from './Document.svelte';
  import DocumentShare from './DocumentShare.svelte';
  import Folder from './Folder.svelte';

  const app = getAppContext();

  const entitiesQuery = createQuery(
    graphql(`
      query DashboardLayout_ShareModal_Query($entityIds: [ID!]!) {
        entities(entityIds: $entityIds) {
          id
          type

          node {
            __typename

            ... on Folder {
              id

              ...DashboardLayout_Share_Folder_folder
            }

            ... on Document {
              id

              ...DashboardLayout_Share_Document_document
            }
          }
        }
      }
    `),
    () => ({ entityIds: app.state.shareOpen }),
    () => ({ skip: app.state.shareOpen.length === 0 }),
  );

  const entities = $derived(entitiesQuery.data?.entities ?? []);
  const singleDocumentEntityId = $derived(
    app.state.shareOpen.length === 1 && entities.length === 1 && entities[0].type === 'DOCUMENT' ? entities[0].id : null,
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

              publication {
                id
                state
              }

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

    const document = singleDocument;
    if (!document) return;

    untrack(() => {
      if (steppedFor === document.id) return;
      steppedFor = document.id;
      step = document.publication && document.publication.state !== 'UNPUBLISHED' ? 'publish' : 'visibility';
    });
  });

  const modalStyle = $derived(
    singleDocument
      ? step === 'publish'
        ? css.raw({ maxWidth: '880px', height: 'full', maxHeight: '680px', padding: '0', overflow: 'hidden' })
        : css.raw({ maxWidth: '480px', padding: '0' })
      : css.raw({ maxWidth: '400px' }),
  );
</script>

<Modal
  style={modalStyle}
  loading={!loaded}
  onclose={() => {
    app.state.shareOpen = [];
  }}
  open={app.state.shareOpen.length > 0}
>
  {#if loaded}
    {#if singleDocument}
      <DocumentShare document$key={singleDocument} onclose={() => (app.state.shareOpen = [])} bind:step />
    {:else if entitiesQuery.data}
      {@const allFolders = entities.every((e) => e.type === 'FOLDER')}
      {@const allDocuments = entities.every((e) => e.type === 'DOCUMENT')}

      {#if allFolders}
        <Folder folders$key={entitiesQuery.data.entities.map((e) => e.node).filter((e) => e.__typename === 'Folder')} />
      {:else if allDocuments}
        <Document documents$key={entitiesQuery.data.entities.map((e) => e.node).filter((e) => e.__typename === 'Document')} />
      {/if}
    {/if}
  {/if}
</Modal>
