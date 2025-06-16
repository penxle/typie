<script lang="ts">
  import { graphql } from '$graphql';
  import { flex } from '$styled-system/patterns';
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

<div class={flex({ flexDirection: 'column', width: '[100dvw]', minHeight: '[100dvh]', height: 'full', paddingTop: '52px' })}>
  <Header $user={$query.me} />

  {#if $query.entityView.node.__typename === 'PostView'}
    <PostView $entityView={$query.entityView} $user={$query.me} />
  {:else}
    <FolderView $entityView={$query.entityView} />
  {/if}
</div>
