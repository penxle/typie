<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { page } from '$app/state';
  import { hydrateQuery } from '$lib/graphql';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import { currentSpaceSlug } from '../current-space-slug';
  import { spaceHomePath } from '../paths';
  import PinnedPublications from '../PinnedPublications.svelte';
  import SpaceHeader from '../SpaceHeader.svelte';
  import SpaceSidebar from '../SpaceSidebar.svelte';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const site = $derived(query.data.siteView);

  const slug = $derived(currentSpaceSlug());
  const pathname = $derived<string>(page.url.pathname);
  const isHome = $derived(pathname === spaceHomePath(slug));
  const isFolder = $derived(pathname.startsWith(`${spaceHomePath(slug)}/f/`));
  const activeTag = $derived(pathname.startsWith(`${spaceHomePath(slug)}/t/`) ? (page.params.name ?? null) : null);

  const chrome = getUsersiteChrome();
  let identityEl = $state<HTMLDivElement>();
  let viewportHeight = $state(0);

  $effect(() => {
    chrome.identityEls = identityEl ? [identityEl] : [];

    return () => {
      chrome.identityEls = [];
    };
  });
</script>

<svelte:window bind:innerHeight={viewportHeight} />

<svelte:head>
  {#if !site.allowIndexing}
    <meta name="robots" content="noindex, nofollow" />
  {/if}
</svelte:head>

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
    {#if isHome}
      <div bind:this={identityEl} class={css({ display: { base: 'block', lg: 'none' }, marginBottom: '32px' })}>
        <SpaceHeader siteView$key={site} variant="compact" />
      </div>

      {#if site.pinnedPublications.length > 0}
        <div class={css({ marginBottom: '56px' })}>
          <PinnedPublications publications={site.pinnedPublications} />
        </div>
      {/if}
    {/if}

    {@render children()}
  </section>

  <SpaceSidebar {activeTag} headerBottom={chrome.headerHeight} hiddenOnMobile={isFolder} siteView$key={site} {viewportHeight} />
</div>
