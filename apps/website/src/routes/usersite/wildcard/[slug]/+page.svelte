<script lang="ts">
  import { graphql } from '$graphql';
  import FolderView from './FolderView.svelte';
  import Header from './Header.svelte';
  import PostView from './PostView.svelte';

  const query = graphql(`
    query UsersiteWildcardSlugPage_Query($origin: String!, $slug: String!) {
      me {
        id

        ...UsersiteWildcardSlugPage_Header_user
        ...UsersiteWildcardSlugPage_PostView_user
      }

      entityView(origin: $origin, slug: $slug) {
        id

        node {
          __typename
        }

        ...UsersiteWildcardSlugPage_FolderView_entityView
        ...UsersiteWildcardSlugPage_PostView_entityView
      }
    }
  `);
</script>

<Header $user={$query.me} />

{#if $query.entityView.node.__typename === 'PostView'}
  <PostView $entityView={$query.entityView} $user={$query.me} />
{:else}
  <FolderView $entityView={$query.entityView} />
{/if}
