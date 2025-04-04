<script lang="ts">
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id
        email
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

<div>{$query.me.email}</div>
<button onclick={handleLogout} type="button">로그아웃</button>
