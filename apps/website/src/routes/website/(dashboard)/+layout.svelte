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
                title
              }
            }

            children {
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
                  title
                }
              }

              children {
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
                    title
                  }
                }
              }
            }
          }

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

          entities {
            id
            slug
            order

            node {
              ... on Folder {
                id
                name
              }
              ... on Post {
                id
                title
              }
            }

            children {
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
                  id
                  title
                }
              }

              children {
                id
                slug
                order

                node {
                  ... on Folder {
                    id
                    name
                  }

                  ... on Post {
                    id
                    title
                  }
                }
              }
            }
          }
        }

        ... on Entity {
          id

          node {
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

          children {
            id
            order

            children {
              id
              order
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
  <Sidebar $user={$query.me} entities={$query.me.sites[0].entities} />

  <div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflowY: 'auto' })}>
    {@render children()}
  </div>
</div>

<CommandPalette $site={$query.me.sites[0]} />
