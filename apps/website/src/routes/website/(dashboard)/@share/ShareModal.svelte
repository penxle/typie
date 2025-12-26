<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { graphql } from '$graphql';
  import Document from './Document.svelte';
  import Folder from './Folder.svelte';
  import Post from './Post.svelte';

  const entitiesQuery = graphql(`
    query DashboardLayout_ShareModal_Query($entityIds: [ID!]!) @client {
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
  `);

  const app = getAppContext();
  let loaded = $state(false);

  const load = async () => {
    loaded = false;

    if (app.state.shareOpen.length > 0) {
      await entitiesQuery.load({ entityIds: app.state.shareOpen });
      loaded = true;
    }
  };

  $effect(() => {
    load();
  });
</script>

<Modal
  style={css.raw({
    maxWidth: '400px',
  })}
  loading={!loaded}
  onclose={() => {
    app.state.shareOpen = [];
    loaded = false;
  }}
  open={app.state.shareOpen.length > 0}
>
  {#if loaded && $entitiesQuery}
    {@const entities = $entitiesQuery.entities}
    {@const allFolders = entities.every((e) => e.type === 'FOLDER')}
    {@const allPosts = entities.every((e) => e.type === 'POST')}
    {@const allDocuments = entities.every((e) => e.type === 'DOCUMENT')}

    {#if allFolders}
      <Folder $folders={$entitiesQuery.entities.map((e) => e.node).filter((e) => e.__typename === 'Folder')} />
    {:else if allPosts}
      <Post $posts={$entitiesQuery.entities.map((e) => e.node).filter((e) => e.__typename === 'Post')} />
    {:else if allDocuments}
      <Document $documents={$entitiesQuery.entities.map((e) => e.node).filter((e) => e.__typename === 'Document')} />
    {/if}
  {/if}
</Modal>
