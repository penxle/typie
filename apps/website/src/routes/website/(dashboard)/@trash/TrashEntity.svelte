<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { graphql } from '$mearie';
  import TrashDocument from './TrashDocument.svelte';
  import TrashFolder from './TrashFolder.svelte';
  import type { DashboardLayout_TrashTree_TrashEntity_entity$key } from '$mearie';

  type Props = {
    entity$key: DashboardLayout_TrashTree_TrashEntity_entity$key;
    onChange?: () => void;
  };

  let { entity$key, onChange }: Props = $props();

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
</script>

{#if entity.data.node.__typename === 'Folder'}
  <TrashFolder folder$key={entity.data.node} {onChange} />
{:else if entity.data.node.__typename === 'Document'}
  <TrashDocument document$key={entity.data.node} {onChange} />
{/if}
