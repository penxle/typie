<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { buildEntityTree } from '$lib/admin-entity-tree';
  import { AdminEntityTree } from '$lib/components/admin';
  import { graphql } from '$mearie';

  type Props = {
    siteId: string;
    includeDeleted: boolean;
  };

  let { siteId, includeDeleted }: Props = $props();

  const query = createQuery(
    graphql(`
      query AdminUserSiteEntityTree_Query($siteId: String!, $includeDeleted: Boolean!) {
        adminSiteEntities(siteId: $siteId, includeDeleted: $includeDeleted) {
          id
          type
          state

          parent {
            id
          }

          node {
            __typename

            ... on Document {
              title
            }

            ... on Folder {
              name
            }
          }
        }
      }
    `),
    () => ({ siteId, includeDeleted }),
  );
</script>

{#if query.data}
  <AdminEntityTree nodes={buildEntityTree([...query.data.adminSiteEntities])} />
{/if}
