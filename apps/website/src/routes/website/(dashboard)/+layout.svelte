<script lang="ts">
  import { graphql } from '$graphql';
  import { setupAppContext } from '$lib/context';
  import { flex } from '$styled-system/patterns';
  import Sidebar from './Sidebar.svelte';

  let { children } = $props();

  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id

        ...DashboardLayout_Sidebar_user

        sites {
          id

          entities {
            __typename
            id
            slug
            order

            node {
              ... on Folder {
                __typename
                id
                name
              }

              ... on Post {
                __typename
                id

                content {
                  __typename
                  id
                  title
                }
              }
            }
          }
        }
      }
    }
  `);

  setupAppContext();
</script>

<div class={flex({ position: 'relative', alignItems: 'flex-start', height: 'screen' })}>
  <Sidebar $user={$query.me} entities={$query.me.sites[0].entities} />

  <div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full' })}>
    {@render children()}
  </div>
</div>
