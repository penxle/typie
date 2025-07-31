<script lang="ts">
  import { fragment, graphql } from '$graphql';
  import Canvas from './Canvas.svelte';
  import Folder from './Folder.svelte';
  import Post from './Post.svelte';
  import type { DashboardLayout_EntityTree_Entity_entity, DashboardLayout_EntityTree_Folder_entity } from '$graphql';

  type Props = {
    $entity: DashboardLayout_EntityTree_Entity_entity;
  };

  let { $entity: _entity }: Props = $props();

  const entity = fragment(
    _entity,
    graphql(`
      fragment DashboardLayout_EntityTree_Entity_entity on Entity {
        id
        depth

        node {
          __typename

          ... on Folder {
            id
            ...DashboardLayout_EntityTree_Folder_folder
          }

          ... on Post {
            id
            ...DashboardLayout_EntityTree_Post_post
          }

          ... on Canvas {
            id
            ...DashboardLayout_EntityTree_Canvas_canvas
          }
        }
      }
    `),
  );

  const children = $derived(($entity as unknown as { children: DashboardLayout_EntityTree_Folder_entity[] }).children ?? []);
</script>

{#if $entity.node.__typename === 'Folder'}
  <Folder $entities={children} $folder={$entity.node} />
{:else if $entity.node.__typename === 'Post'}
  <Post $post={$entity.node} />
{:else if $entity.node.__typename === 'Canvas'}
  <Canvas $canvas={$entity.node} />
{/if}
