<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { setupAppContext } from '@typie/ui/context';
  import { onMount } from 'svelte';
  import { graphql } from '$graphql';
  import Home from './@pages/Home.svelte';
  import Sidebar from './Sidebar.svelte';
  import TabBar from './TabBar.svelte';
  import { tabState } from './tabs.svelte';

  const query = graphql(`
    query App_Query {
      me @required {
        id

        ...Sidebar_user
      }
    }
  `);

  onMount(() => {
    if (tabState.tabs.length === 0) {
      tabState.add(Home, {});
    }
  });

  setupAppContext($query.me.id);
</script>

<div class={flex({ position: 'relative', width: '[100vw]', height: '[100vh]', backgroundColor: 'surface.subtle' })}>
  <Sidebar $user={$query.me} />

  <div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full' })}>
    <TabBar />

    <div class={css({ flexGrow: '1', position: 'relative', marginRight: '8px', marginBottom: '8px', overflow: 'hidden' })}>
      {#each tabState.tabs as tab (tab.id)}
        <div
          class={css({
            height: 'full',
            borderWidth: '[0.5px]',
            borderRadius: '4px',
            backgroundColor: 'surface.default',
            boxShadow: '[0 3px 6px -2px {colors.shadow.default/3}, 0 1px 1px {colors.shadow.default/5}]',
            overflow: 'hidden',
          })}
          hidden={!tab.active}
        >
          <tab.component tabId={tab.id} {...tab.props} />
        </div>
      {/each}
    </div>
  </div>
</div>
