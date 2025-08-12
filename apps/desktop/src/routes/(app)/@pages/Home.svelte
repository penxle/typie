<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { store } from '$lib/store';
  import { tabState } from '../tabs.svelte';
  import Counter from './Counter.svelte';

  type Props = {
    tabId: string;
  };

  const { tabId }: Props = $props();

  const query = graphql(`
    query HomePage_Query @client {
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

  onMount(() => {
    tabState.setTitle(tabId, '홈');
  });
</script>

<main class={center({ flexDirection: 'column', gap: '24px', height: 'full' })}>
  <div class={css({ textAlign: 'center' })}>
    <h1 class={css({ fontSize: '24px', fontWeight: 'bold', marginBottom: '8px' })}>환영합니다!</h1>
    <div class={css({ color: 'gray.600' })}>{$query?.me?.email}</div>
  </div>

  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
    <Button onclick={() => tabState.navigate(tabId, Counter, {})} variant="secondary">카운터 데모</Button>
    <Button onclick={logout}>로그아웃</Button>
  </div>

  <div
    class={css({
      padding: '16px',
      backgroundColor: 'gray.50',
      borderRadius: '8px',
      fontSize: '13px',
      color: 'gray.600',
      maxWidth: '400px',
      textAlign: 'center',
    })}
  >
    탭 ID: {tabId}
  </div>
</main>
