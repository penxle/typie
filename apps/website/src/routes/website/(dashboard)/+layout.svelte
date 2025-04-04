<script lang="ts">
  import { onMount } from 'svelte';
  import { graphql } from '$graphql';
  import { expandSidebar } from '$lib/stores/global-stores';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import SidebarContainer from './SidebarContainer.svelte';
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

  let mounted = $state(false);

  onMount(() => {
    if ($expandSidebar === null) $expandSidebar = true;

    mounted = true;
  });

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

{#if mounted}
  <div class={css({ display: 'flex', flexDirection: 'column', flexGrow: '1', height: 'screen' })}>
    <div class={flex({ align: 'flex-start', height: 'full' })}>
      <SidebarContainer {items} />

      <div class={css({ width: 'full' })}>
        {@render children()}
      </div>
    </div>
  </div>
{/if}
