<script lang="ts">
  import { fragment, graphql } from '$graphql';
  import TrashDocument from './TrashDocument.svelte';
  import TrashFolder from './TrashFolder.svelte';
  import TrashPost from './TrashPost.svelte';
  import type { DashboardLayout_TrashTree_TrashEntity_entity, DashboardLayout_TrashTree_TrashFolder_entity } from '$graphql';

  type Props = {
    $entity: DashboardLayout_TrashTree_TrashEntity_entity;
  };

  let { $entity: _entity }: Props = $props();

  const entity = fragment(
    _entity,
    graphql(`
      fragment DashboardLayout_TrashTree_TrashEntity_entity on Entity {
        id

        node {
          __typename

          ... on Folder {
            id
            ...DashboardLayout_TrashTree_TrashFolder_folder
          }

          ... on Post {
            id
            ...DashboardLayout_TrashTree_TrashPost_post
          }

          ... on Document {
            id
            ...DashboardLayout_TrashTree_TrashDocument_document
          }
        }
      }
    `),
  );

  const children = $derived(
    ($entity as unknown as { deletedChildren: DashboardLayout_TrashTree_TrashFolder_entity[] }).deletedChildren ?? [],
  );
</script>

{#if $entity.node.__typename === 'Folder'}
  <TrashFolder $entities={children} $folder={$entity.node} />
{:else if $entity.node.__typename === 'Post'}
  <TrashPost $post={$entity.node} />
{:else if $entity.node.__typename === 'Document'}
  <TrashDocument $document={$entity.node} />
{/if}
