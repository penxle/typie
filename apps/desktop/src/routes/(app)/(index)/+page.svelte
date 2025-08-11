<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { store } from '$lib/store';

  const query = graphql(`
    query Index_Query {
      me {
        id
        email
      }
    }
  `);

  const logout = async () => {
    await store.delete('access_token');
    goto('/auth/login');
  };
</script>

<main class={center({ flexDirection: 'column', gap: '16px', height: 'full' })}>
  <div>{$query.me?.email}</div>
  <Button onclick={logout}>로그아웃</Button>
</main>
