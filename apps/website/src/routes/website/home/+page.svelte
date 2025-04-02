<script lang="ts">
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PageList from '../home/PageList.svelte';
  import type { Item } from './types';

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
  let items: Item[] = [
    {
      id: '1',
      type: 'folder',
      title: '폴더1',
      children: [
        { id: '2', type: 'page', title: '페이지1' },
        { id: '3', type: 'page', title: '페이지2' },
        { id: '13', type: 'page', title: '페이지3' },
      ],
    },
    {
      id: '4',
      type: 'folder',
      title: '폴더2',
      children: [
        { id: '5', type: 'page', title: '페이지3' },
        { id: '6', type: 'page', title: '페이지4' },
        {
          id: '7',
          type: 'folder',
          title: '폴더3',
          children: [
            { id: '8', type: 'page', title: '페이지5' },
            { id: '9', type: 'page', title: '페이지6' },
            {
              id: '10',
              type: 'folder',
              title: '폴더4',
              children: [
                { id: '11', type: 'page', title: '페이지7' },
                { id: '12', type: 'page', title: '페이지8' },
              ],
            },
          ],
        },
      ],
    },
    {
      id: '13',
      type: 'folder',
      title: '폴더3',
      children: [{ id: '14', type: 'page', title: '페이지9' }],
    },
  ];
</script>

<div class={flex({ align: 'flex-start', height: 'full' })}>
  <div class={css({ flex: 'none', backgroundColor: 'gray.100', width: '300px', height: 'full', overflowY: 'auto' })}>
    <nav class={css({ position: 'sticky', top: '0' })}>
      <p>홈</p>
      <p>검색</p>
      <p>알림</p>
      <p>설정</p>
    </nav>

    <hr class={css({ marginY: '20px', border: 'none', height: '1px', width: 'full', backgroundColor: 'gray.900' })} />

    <div>
      <p>보관함</p>

      <PageList {items} />
    </div>
  </div>

  <div class={css({ width: 'full' })}>
    <div>{$query.me.email}</div>
    <button onclick={handleLogout} type="button">로그아웃</button>
  </div>
</div>
