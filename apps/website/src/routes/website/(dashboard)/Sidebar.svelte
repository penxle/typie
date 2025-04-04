<script lang="ts">
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import Logo from '$assets/logos/logo.svg?component';
  import { Icon } from '$lib/components';
  import { expandSidebar } from '$lib/stores';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
  import type { Item } from './types';

  type Props = {
    items: Item[];
    sidebarPopoverVisible: boolean;
  };

  let { items, sidebarPopoverVisible = $bindable() }: Props = $props();
</script>

<div class={flex({ align: 'center', justify: 'space-between' })}>
  <Logo class={css({ height: '32px', flex: 'none' })} />

  <div class={flex({ align: 'center' })}>
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
    <button type="button">새글</button>
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
