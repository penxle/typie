<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { graphql } from '$mearie';
  import Document from './Document.svelte';
  import Folder from './Folder.svelte';
  import type { DashboardLayout_EntityTree_Entity_entity$key } from '$mearie';

  type Props = {
    entity$key: DashboardLayout_EntityTree_Entity_entity$key;
  };

  let { entity$key }: Props = $props();

  const entity = createFragment(
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

          ... on Document {
            id
            ...DashboardLayout_EntityTree_Document_document
          }
        }
      }
    `),
    () => entity$key,
  );
</script>

{#if entity.data.node.__typename === 'Folder'}
  <Folder folder$key={entity.data.node} />
{:else if entity.data.node.__typename === 'Document'}
  <Document document$key={entity.data.node} />
{/if}
