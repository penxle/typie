<script lang="ts">
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import TopBar from '../TopBar.svelte';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id
        email

        sites {
          id

          entities {
            id

            node {
              ... on Folder {
                id
                name
              }

              ... on Post {
                id

                content {
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

  const logout = graphql(`
    mutation HomePage_Logout_Mutation {
      logout
    }
  `);

  const handleLogout = async () => {
    await logout();
    await goto('/');
  };
</script>

<TopBar />

<div>{$query.me.email}</div>
<div>
  <button onclick={handleLogout} type="button">로그아웃</button>
</div>

<pre>
  {#each $query.me.sites as site (site.id)}
    <pre>{JSON.stringify(site, null, 2)}</pre>
  {/each}
</pre>
