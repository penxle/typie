<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { graphql } from '$mearie';
  import TrashDocument from './TrashDocument.svelte';
  import TrashFolder from './TrashFolder.svelte';
  import type { DashboardLayout_TrashTree_TrashEntity_entity$key, DashboardLayout_TrashTree_TrashFolder_entity$key } from '$mearie';

  type Props = {
    entity$key: DashboardLayout_TrashTree_TrashEntity_entity$key;
  };

  let { entity$key }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment DashboardLayout_TrashTree_TrashEntity_entity on Entity {
        id

        node {
          __typename

          ... on Folder {
            id
            ...DashboardLayout_TrashTree_TrashFolder_folder
          }

          ... on Document {
            id
            ...DashboardLayout_TrashTree_TrashDocument_document
          }
        }
      }
    `),
    () => entity$key,
  );

  const children = $derived(
    (entity.data as unknown as { deletedChildren: DashboardLayout_TrashTree_TrashFolder_entity$key[] }).deletedChildren ?? [],
  );
</script>

{#if entity.data.node.__typename === 'Folder'}
  <TrashFolder entities$key={children} folder$key={entity.data.node} />
{:else if entity.data.node.__typename === 'Document'}
  <TrashDocument document$key={entity.data.node} />
{/if}
