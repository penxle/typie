<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { graphql } from '$graphql';
  import { Modal } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import Folder from './Folder.svelte';
  import Post from './Post.svelte';

  const query = graphql(`
    query DashboardLayout_ShareModal_Query($entityId: ID!) @client {
      entity(entityId: $entityId) {
        id

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
        }
      }
    }
  `);

  const app = getAppContext();
  let loaded = $state(false);

  const load = async () => {
    if (app.state.shareOpen) {
      loaded = false;
      await query.load({ entityId: app.state.shareOpen });
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
  loading={!loaded || !query}
  onclose={() => {
    app.state.shareOpen = false;
    loaded = false;
  }}
  open={!!app.state.shareOpen}
>
  {#if loaded && $query}
    {#key $query.entity.id}
      {#if $query.entity.node.__typename === 'Post'}
        <Post $post={$query.entity.node} />
      {:else if $query.entity.node.__typename === 'Folder'}
        <Folder $folder={$query.entity.node} />
      {/if}
    {/key}
  {/if}
</Modal>
