<script lang="ts">
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import PenLineIcon from '~icons/lucide/pen-line';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { expandSidebar } from '$lib/stores';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
  import type { DashboardLayout_Sidebar_user } from '$graphql';
  import type { Item } from './types';

  type Props = {
    $user: DashboardLayout_Sidebar_user;
    items: Item[];
    sidebarPopoverVisible: boolean;
  };

  let { $user: _user, items, sidebarPopoverVisible = $bindable() }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id

        sites {
          id
        }
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Sidebar_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id
      }
    }
  `);
</script>

<div class={flex({ align: 'center', justify: 'space-between' })}>
  <Logo class={css({ height: '32px', flex: 'none' })} />

  <div class={flex({ align: 'center', gap: '4px' })}>
    {#if $expandSidebar}
      <button
        onclick={() => {
          $expandSidebar = false;
          sidebarPopoverVisible = false;
        }}
        type="button"
      >
        <Icon icon={ChevronsLeftIcon} />
      </button>
    {/if}

    <button
      onclick={async () => {
        const resp = await createPost({
          siteId: $user.sites[0].id,
        });

        await goto(`/editor/${resp.id}`);
      }}
      type="button"
    >
      <Icon icon={PenLineIcon} />
    </button>
  </div>
</div>

<div class={css({ flex: 'none' })}>
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
