<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { graphql } from '$mearie';
  import Document from './Document.svelte';
  import Folder from './Folder.svelte';
  import Post from './Post.svelte';

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

            ... on Post {
              id

              ...DashboardLayout_Share_Post_post
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

  const loaded = $derived(app.state.shareOpen.length > 0 && !!entitiesQuery.data && !entitiesQuery.loading);
</script>

<Modal
  style={css.raw({
    maxWidth: '400px',
  })}
  loading={!loaded}
  onclose={() => {
    app.state.shareOpen = [];
  }}
  open={app.state.shareOpen.length > 0}
>
  {#if loaded && entitiesQuery.data}
    {@const entities = entitiesQuery.data.entities}
    {@const allFolders = entities.every((e) => e.type === 'FOLDER')}
    {@const allPosts = entities.every((e) => e.type === 'POST')}
    {@const allDocuments = entities.every((e) => e.type === 'DOCUMENT')}

    {#if allFolders}
      <Folder folders$key={entitiesQuery.data.entities.map((e) => e.node).filter((e) => e.__typename === 'Folder')} />
    {:else if allPosts}
      <Post posts$key={entitiesQuery.data.entities.map((e) => e.node).filter((e) => e.__typename === 'Post')} />
    {:else if allDocuments}
      <Document documents$key={entitiesQuery.data.entities.map((e) => e.node).filter((e) => e.__typename === 'Document')} />
    {/if}
  {/if}
</Modal>
