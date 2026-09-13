<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { tick } from 'svelte';
  import { cubicIn, cubicOut } from 'svelte/easing';
  import { MediaQuery } from 'svelte/reactivity';
  import { fly } from 'svelte/transition';
  import { afterNavigate, beforeNavigate, goto } from '$app/navigation';
  import { page } from '$app/state';
  import { hydrateQuery } from '$lib/graphql';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import BackBar from '../BackBar.svelte';
  import { currentSpaceSlug } from '../current-space-slug';
  import { seriesListPath, spaceHomePath, tagPath } from '../paths';
  import PinnedPublications from '../PinnedPublications.svelte';
  import SpaceHeader from '../SpaceHeader.svelte';
  import SpaceRail from '../SpaceRail.svelte';
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
  const collectionId = $derived(pathname.startsWith('/s/') ? (page.params.id ?? null) : null);
  const collection = $derived(collectionId ? (space.collections.find((item) => item.id === collectionId) ?? null) : null);
  const tag = $derived(tab === 'tags' ? (page.params.name ?? null) : null);
  const desktopMode = $derived(collection ? 'series' : tag ? 'tag' : 'posts');

  const order = (path: string) => {
    if (path === '/s') return 1;
    if (path.startsWith('/s/')) return 1.5;
    if (path.startsWith('/t/')) return 2;
    return 0;
  };

  const chrome = getUsersiteChrome();
  let identityEl = $state<HTMLDivElement>();
  let railEl = $state<HTMLElement>();

  $effect(() => {
    chrome.identityEls = [identityEl, railEl].filter((el) => el !== undefined);

    return () => {
      chrome.identityEls = [];
    };
  });

  let direction = $state(1);
  let keepListInView = false;
  let listAnchor = $state<HTMLDivElement>();

  const desktop = new MediaQuery('(min-width: 1024px)');
  const motion = $derived(prefersReducedMotion.current || desktop.current ? 0 : 1);
  const enter = $derived({ x: 16 * direction * motion, duration: 220 * motion, easing: cubicOut });
  const leave = $derived({ x: -8 * direction * motion, duration: 110 * motion, easing: cubicIn });

  beforeNavigate(({ from, to }) => {
    if (!from || !to) return;
    direction = Math.sign(order(relative(to.url.pathname)) - order(relative(from.url.pathname))) || 1;
    const anchor = listAnchor;
    keepListInView = !!anchor && (anchor.querySelector('[data-stuck]') !== null || anchor.getBoundingClientRect().top < 0);
  });

  afterNavigate(() => {
    if (!keepListInView) return;
    keepListInView = false;
    void tick().then(() => listAnchor?.scrollIntoView({ block: 'start' }));
  });

  const isDesktop = () => window.matchMedia('(min-width: 1024px)').matches;
  const navigate = (path: string) => {
    if (path === page.url.pathname) return;
    void goto(path, { noScroll: true, keepFocus: true });
  };

  const selectTab = (next: SpaceHomeTab) => {
    if (next === 'tags') {
      const first = space.tags[0];
      if (first) navigate(tagPath(slug, first.name));
      return;
    }
    navigate(next === 'series' ? seriesListPath(slug) : spaceHomePath(slug));
  };

  const leaveCollection = () => navigate(isDesktop() ? spaceHomePath(slug) : seriesListPath(slug));
</script>

<svelte:head>
  {#if !space.allowIndexing}
    <meta name="robots" content="noindex, nofollow" />
  {/if}
</svelte:head>

<div class={flex({ justifyContent: 'center', width: 'full', minHeight: 'full' })}>
  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: { base: '1fr', lg: '[minmax(0, 680px) 240px]' },
      gap: { base: '28px', lg: '48px' },
      justifyContent: 'center',
      alignItems: 'start',
      width: 'full',
      maxWidth: '1064px',
      paddingX: { base: '20px', md: '40px' },
      paddingTop: { base: '28px', lg: '64px' },
      paddingBottom: '120px',
    })}
  >
    <aside
      bind:this={railEl}
      class={css({
        display: { base: 'none', lg: 'block' },
        order: '2',
        position: 'sticky',
        top: '[calc(var(--usersite-sticky-header-bottom, 0px) + 24px)]',
      })}
    >
      <SpaceRail activeCollectionId={collection?.id ?? null} activeTag={tag} spaceView$key={space} />
    </aside>

    <section class={css({ minWidth: '0' })}>
      <div bind:this={identityEl} class={css({ display: { base: 'block', lg: 'none' } })}>
        <SpaceHeader spaceView$key={space} variant="compact" />
      </div>

      {#if space.pinnedPublications.length > 0}
        <div
          class={css({ display: { base: 'block', lg: desktopMode === 'posts' ? 'block' : 'none' }, marginTop: { base: '28px', lg: '0' } })}
        >
          <PinnedPublications dateDisplay={space.dateDisplay} publications={space.pinnedPublications} />
        </div>
      {/if}

      <div
        bind:this={listAnchor}
        class={css({
          marginTop: { base: '32px', lg: desktopMode === 'posts' && space.pinnedPublications.length > 0 ? '48px' : '0' },
          scrollMarginTop: '[calc(var(--usersite-sticky-header-bottom, 0px) + 12px)]',
        })}
      >
        {#if collection}
          <BackBar compactLabel="시리즈" label="글" onBack={leaveCollection} />
        {:else}
          <div class={css({ display: { base: 'block', lg: 'none' } })}>
            <SpaceTabs
              counts={{ posts: space.publicationCount, series: space.collections.length, tags: space.tags.length }}
              onselect={selectTab}
              {tab}
            />
          </div>

          {#if tag}
            <div class={css({ display: { base: 'none', lg: 'block' } })}>
              <BackBar label="글" onBack={() => navigate(spaceHomePath(slug))} />
            </div>
          {:else}
            <div
              class={flex({
                display: { base: 'none', lg: 'flex' },
                alignItems: 'center',
                height: '44px',
                borderBottomWidth: '1px',
                borderColor: 'border.hairline',
              })}
            >
              <h2 class={css({ fontSize: '16px', fontWeight: 'semibold' })}>글</h2>
            </div>
          {/if}
        {/if}

        <div class={css({ display: 'grid', '& > *': { gridArea: '[1 / 1]', minWidth: '0' } })}>
          {#key page.url.pathname}
            <div id={`space-panel-${tab}`} aria-labelledby={`space-tab-${tab}`} role="tabpanel" in:fly={enter} out:fly={leave}>
              {@render children()}
            </div>
          {/key}
        </div>
      </div>
    </section>
  </div>
</div>
