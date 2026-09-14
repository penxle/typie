<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { page } from '$app/state';
  import { hydrateQuery } from '$lib/graphql';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import DiscoverySidebar from '../DiscoverySidebar.svelte';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const feedLayoutQuery = $derived(hydrateQuery(() => data.feedLayoutQuery));

  const chrome = getUsersiteChrome();

  const pathname = $derived<string>(page.url.pathname);
  const currentTag = $derived(pathname.startsWith('/t/') ? decodeURIComponent(pathname.slice(3)) : null);

  let viewportHeight = $state(0);
</script>

<svelte:window bind:innerHeight={viewportHeight} />

<div
  class={css({
    display: 'grid',
    gridTemplateColumns: { base: 'minmax(0, 1fr)', lg: 'minmax(0, 1fr) 300px' },
    gap: '56px',
    alignItems: 'start',
    width: 'full',
    maxWidth: { base: '760px', lg: '1200px' },
    marginX: 'auto',
    paddingTop: { base: '20px', md: '28px' },
    paddingX: { base: '20px', md: '40px' },
    paddingBottom: '120px',
    wordBreak: 'keep-all',
    overflowWrap: 'anywhere',
  })}
>
  <section class={css({ minWidth: '0' })}>
    {@render children()}
  </section>

  <DiscoverySidebar
    {currentTag}
    discovery={feedLayoutQuery.data.discovery}
    headerBottom={chrome.stickyBottom}
    loggedIn={!!query.data.me}
    {viewportHeight}
  />
</div>
