<script lang="ts">
  import { quintInOut } from 'svelte/easing';
  import { scale, slide } from 'svelte/transition';
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { expandSidebar } from '$lib/stores';
  import { css } from '$styled-system/css';
  import Sidebar from './Sidebar.svelte';
  import type { DashboardLayout_SidebarContainer_user } from '$graphql';
  import type { Item } from './types';

  type Props = {
    $user: DashboardLayout_SidebarContainer_user;
    items: Item[];
  };

  let { $user: _user, items }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_SidebarContainer_user on User {
        id
        ...DashboardLayout_Sidebar_user
      }
    `),
  );

  let sidebarPopoverVisible = $state(false);
</script>

{#if $expandSidebar}
  <div
    class={css({
      flex: 'none',
      backgroundColor: 'gray.100',
      minWidth: '300px',
      width: '300px',
      height: 'full',
      overflowY: 'auto',
      '& > div': { width: '300px' },
    })}
    in:scale={{ start: 0.95, duration: 10, opacity: 0.9, easing: quintInOut }}
    out:slide={{ axis: 'x' }}
  >
    <Sidebar {$user} {items} bind:sidebarPopoverVisible />
  </div>
{:else}
  <div
    class={css({
      position: 'fixed',
      top: '0',
      left: '0',
      zIndex: '[1000]',
      width: '30px',
      height: 'full',
    })}
    onpointerenter={() => (sidebarPopoverVisible = true)}
  ></div>

  <div
    class={css(
      {
        position: 'fixed',
        top: '0',
        left: '0',
        zIndex: '[1000]',
        width: '200px',
        height: 'full',
        paddingBottom: '30px',
        transform: 'translateX(-100%)',
        transition: 'transform',
        transitionDuration: '0.3s',
      },
      sidebarPopoverVisible && { transform: 'translateX(0)' },
    )}
    onpointerenter={() => (sidebarPopoverVisible = true)}
    onpointerleave={() => (sidebarPopoverVisible = false)}
  >
    <div
      class={css({
        display: 'flex',
        flexDirection: 'column',
        height: 'full',
      })}
    >
      <button class={css({ backgroundColor: 'white' })} onclick={() => ($expandSidebar = true)} type="button">
        <Icon icon={ChevronsRightIcon} />
      </button>

      <div class={css({ height: 'full', backgroundColor: 'white', boxShadow: 'medium' })}>
        <Sidebar {$user} {items} bind:sidebarPopoverVisible />
      </div>
    </div>
  </div>
{/if}
