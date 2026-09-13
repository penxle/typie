<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';
  import { cleanupBrowserPushForLogout } from '$lib/push';
  import { setUsersiteChrome, UsersiteChrome } from '../../chrome.svelte';
  import ApexHeader from './ApexHeader.svelte';
  import { apexAuthorizeUrl } from './authorize-url';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const chrome = new UsersiteChrome();
  setUsersiteChrome(chrome);

  let stickyHeader = $state<HTMLElement>();
  let stickyHeaderHeight = $state(52);
  let stickyHeaderBottom = $state(52);

  function syncStickyHeaderBottom(): void {
    stickyHeaderBottom = stickyHeader?.getBoundingClientRect().bottom ?? stickyHeaderHeight;
    chrome.stickyBottom = stickyHeaderBottom;
  }

  $effect(() => {
    void stickyHeaderHeight;
    syncStickyHeaderBottom();
  });

  $effect(() => {
    chrome.headerHeight = stickyHeaderHeight;
  });

  let ticking = false;
  const onscroll = () => {
    syncStickyHeaderBottom();
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      ticking = false;
      chrome.sync();
    });
  };

  const onresize = () => {
    syncStickyHeaderBottom();
    chrome.sync();
  };

  afterNavigate(() => {
    chrome.reset();
  });

  onMount(() => {
    chrome.reset();
  });

  const authorizeUrl = $derived(apexAuthorizeUrl(page.url));

  const pathname = $derived<string>(page.url.pathname);
  const initialQuery = $derived(pathname === '/search' ? (page.url.searchParams.get('q') ?? '') : '');

  const logout = () => {
    mixpanel.track('logout', { via: 'header' });
    const logoutUrl = qs.stringifyUrl({
      url: '/logout',
      query: {
        redirect_uri: page.url.href,
      },
    });
    void cleanupBrowserPushForLogout().finally(() => location.assign(logoutUrl));
  };
</script>

<svelte:window {onresize} {onscroll} />

<div style:--usersite-sticky-header-bottom={`${stickyHeaderBottom}px`} class={css({ display: 'contents' })}>
  <header
    bind:this={stickyHeader}
    class={flex({
      flexDirection: 'column',
      position: 'sticky',
      top: '0',
      zIndex: '50',
    })}
    bind:clientHeight={stickyHeaderHeight}
  >
    <AdminImpersonateBanner query$key={query.data} />

    <ApexHeader {authorizeUrl} {initialQuery} onLogout={logout} user$key={query.data.me} />
  </header>

  <main class={flex({ flexDirection: 'column', flex: '1' })}>
    {@render children()}
  </main>
</div>
