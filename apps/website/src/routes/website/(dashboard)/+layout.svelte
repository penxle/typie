<script lang="ts">
  import { graphql } from '$graphql';
  import { setupAppContext } from '$lib/context';
  import { flex } from '$styled-system/patterns';
  import Sidebar from './Sidebar.svelte';
  import type { Item } from './types';

  let { children } = $props();

  const query = graphql(`
    query DashboardLayout_Query {
      me @required {
        id

        ...DashboardLayout_Sidebar_user
      }
    }
  `);

  setupAppContext();

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

<div class={flex({ position: 'relative', alignItems: 'flex-start', height: 'screen' })}>
  <Sidebar $user={$query.me} {items} />

  <div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full' })}>
    {@render children()}
  </div>
</div>
