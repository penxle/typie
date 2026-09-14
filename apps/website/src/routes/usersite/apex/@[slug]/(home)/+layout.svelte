<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { cubicOut } from 'svelte/easing';
  import { MediaQuery } from 'svelte/reactivity';
  import { fly } from 'svelte/transition';
  import { beforeNavigate, goto } from '$app/navigation';
  import { page } from '$app/state';
  import { hydrateQuery } from '$lib/graphql';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import { currentSpaceSlug } from '../current-space-slug';
  import { seriesListPath, spaceHomePath, tagPath } from '../paths';
  import PinnedPublications from '../PinnedPublications.svelte';
  import SpaceHeader from '../SpaceHeader.svelte';
  import SpaceSidebar from '../SpaceSidebar.svelte';
  import SpaceTabs from '../SpaceTabs.svelte';
  import type { SpaceHomeTab } from '../SpaceTabs.svelte';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const space = $derived(query.data.spaceView);

  const slug = $derived(currentSpaceSlug());
  const base = $derived(spaceHomePath(slug));
  const relative = (path: string) => (path.startsWith(`${base}/`) ? path.slice(base.length) : '/');
  const pathname = $derived<string>(relative(page.url.pathname));
  const tab = $derived<SpaceHomeTab>(
    pathname === '/s' || pathname.startsWith('/s/') ? 'series' : pathname.startsWith('/t/') ? 'tags' : 'posts',
  );
  const isHome = $derived(pathname === '/');
  const collectionPermalink = $derived(pathname.startsWith('/s/') ? (page.params.permalink ?? null) : null);
  const activeCollectionId = $derived(
    collectionPermalink ? (space.collections.find((item) => item.permalink === collectionPermalink)?.id ?? null) : null,
  );
  const activeTag = $derived(tab === 'tags' ? (page.params.name ?? null) : null);

  const order = (path: string) => {
    if (path === '/s') return 1;
    if (path.startsWith('/s/')) return 1.5;
    if (path.startsWith('/t/')) return 2;
    return 0;
  };

  const chrome = getUsersiteChrome();
  let identityEl = $state<HTMLDivElement>();
  let viewportHeight = $state(0);
  let tabsHeight = $state(45);

  $effect(() => {
    chrome.identityEls = identityEl ? [identityEl] : [];

    return () => {
      chrome.identityEls = [];
    };
  });

  let direction = $state(1);

  const desktop = new MediaQuery('(min-width: 1024px)');
  const motion = $derived(prefersReducedMotion.current || desktop.current ? 0 : 1);
  const enter = $derived({ x: 16 * direction * motion, duration: 220 * motion, easing: cubicOut });

  beforeNavigate(({ from, to }) => {
    if (!from || !to) return;
    direction = Math.sign(order(relative(to.url.pathname)) - order(relative(from.url.pathname))) || 1;
  });

  const navigate = (path: string) => {
    if (path === page.url.pathname) return;
    void goto(path, { keepFocus: true });
  };

  const selectTab = (next: SpaceHomeTab) => {
    if (next === 'tags') {
      const first = space.tags[0];
      if (first) navigate(tagPath(slug, first.name));
      return;
    }
    navigate(next === 'series' ? seriesListPath(slug) : spaceHomePath(slug));
  };
</script>

<svelte:window bind:innerHeight={viewportHeight} />

<svelte:head>
  {#if !space.allowIndexing}
    <meta name="robots" content="noindex, nofollow" />
  {/if}
</svelte:head>

<SpaceTabs
  counts={{ posts: space.publicationCount, series: space.collections.length, tags: space.tags.length }}
  onselect={selectTab}
  {tab}
  bind:height={tabsHeight}
/>

<div
  style:--usersite-space-tabs-height={`${tabsHeight}px`}
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
    {#key page.url.pathname}
      <div id={`space-panel-${tab}`} aria-labelledby={`space-tab-${tab}`} role="tabpanel" in:fly={enter}>
        {#if isHome}
          <div bind:this={identityEl} class={css({ display: { base: 'block', lg: 'none' }, marginBottom: '32px' })}>
            <SpaceHeader spaceView$key={space} variant="compact" />
          </div>

          {#if space.pinnedPublications.length > 0}
            <div class={css({ marginBottom: '56px' })}>
              <PinnedPublications publications={space.pinnedPublications} />
            </div>
          {/if}
        {/if}

        {@render children()}
      </div>
    {/key}
  </section>

  <SpaceSidebar {activeCollectionId} {activeTag} headerBottom={chrome.headerHeight + tabsHeight} spaceView$key={space} {viewportHeight} />
</div>
