<script lang="ts">
  import { graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
  import type { Item } from './types';

  let { children } = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id
        email
      }
    }
  `);

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

<div class={css({ display: 'flex', flexDirection: 'column', flexGrow: '1', height: 'screen' })}>
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
      {@render children()}
    </div>
  </div>
</div>
