<script lang="ts">
  import { untrack } from 'svelte';
  import { graphql } from '$graphql';
  import { setupAppContext } from '$lib/context';
  import { flex } from '$styled-system/patterns';
  import CommandPalette from './CommandPalette.svelte';
  import Sidebar from './Sidebar.svelte';

  let { children } = $props();

  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id

        sites {
          id
          name

          ...DashboardLayout_CommandPalette_site
        }

        ...DashboardLayout_Sidebar_user
      }
    }
  `);

  const siteUpdateStream = graphql(`
    subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
      siteUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          ...DashboardLayout_EntityTree_site
        }

        ... on Entity {
          id

          node {
            __typename

            ... on Folder {
              id
              name
            }

            ... on Post {
              id
              title

              characterCountChange {
                additions
                deletions
              }
            }
          }
        }
      }
    }
  `);

  setupAppContext();

  $effect(() => {
    return untrack(() => {
      const unsubscribe = siteUpdateStream.subscribe({ siteId: $query.me.sites[0].id });

      return () => {
        unsubscribe();
      };
    });
  });
</script>

<div class={flex({ position: 'relative', alignItems: 'flex-start', height: 'screen' })}>
  <Sidebar $user={$query.me} />

  <div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflowY: 'auto' })}>
    {@render children()}
  </div>
</div>

<CommandPalette $site={$query.me.sites[0]} />
