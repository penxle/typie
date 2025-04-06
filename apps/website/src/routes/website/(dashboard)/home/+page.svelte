<script lang="ts">
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';
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

<div class={center({ flexDirection: 'column', flexGrow: '1', width: 'full' })}>
  <div>{$query.me.email}</div>
  <div>
    <button onclick={handleLogout} type="button">로그아웃</button>
  </div>
</div>
