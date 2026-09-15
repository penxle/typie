<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { MediaQuery } from 'svelte/reactivity';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import SearchIcon from '~icons/lucide/search';
  import XIcon from '~icons/lucide/x';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { chromeHidden, readingProgress, titleSlot } from '$lib/usersite/post-chrome';
  import AccountMenu from '../../AccountMenu.svelte';
  import { getUsersiteChrome } from '../../chrome.svelte';
  import ApexSearchPanel from './ApexSearchPanel.svelte';
  import ApexWordmark from './ApexWordmark.svelte';
  import { discoveryHomePath, discoveryLatestPath, discoveryTagsPath } from './paths';
  import type { UsersiteHeader_user$key } from '$mearie';

  type Props = {
    user$key: UsersiteHeader_user$key | null | undefined;
    authorizeUrl: string;
    onLogout: () => void;
    initialQuery?: string;
  };

  let { user$key, authorizeUrl, onLogout, initialQuery = '' }: Props = $props();

  type TabId = 'home' | 'latest' | 'tags';

  const TABS: { id: TabId; label: string; href: string }[] = [
    { id: 'home', label: '홈', href: discoveryHomePath },
    { id: 'latest', label: '최신', href: discoveryLatestPath },
    { id: 'tags', label: '태그', href: discoveryTagsPath },
  ];

  const chrome = getUsersiteChrome();
  const desktop = new MediaQuery('(min-width: 1024px)');

  let accountMenuOpen = $state(false);
  let searchOpen = $state(false);
  let scrolled = $state(false);
  let query = $state(initialQuery);
  let searchButton = $state<HTMLButtonElement>();
  let root = $state<HTMLDivElement>();
  let slot = $state<'' | 'title'>('');
  let progress = $state(0);

  const isPost = $derived(chrome.post !== null);
  const eyebrow = $derived(chrome.post?.eyebrow ?? null);
  const hidden = $derived(chromeHidden({ post: isPost, retreat: chrome.retreat, desktop: desktop.current }));

  $effect(() => {
    void chrome.tick;
    void chrome.post;
    void chrome.titleEl;
    void chrome.stickyBottom;
    measurePost();
  });

  const measurePost = () => {
    if (!chrome.post) {
      slot = '';
      progress = 0;
      return;
    }
    slot = titleSlot(chrome.titleEl?.getBoundingClientRect().bottom, chrome.stickyBottom);
    progress = readingProgress(window.scrollY, document.documentElement.scrollHeight, window.innerHeight);
  };

  const scrollToTop = () => {
    window.scrollTo({ top: 0, behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
  };

  const stackItem = css.raw({
    position: 'absolute',
    left: '1/2',
    top: '1/2',
    maxWidth: 'full',
    paddingX: '6px',
    paddingY: '2px',
    fontSize: '14px',
    fontWeight: 'semibold',
    letterSpacing: '-0.01em',
    lineHeight: '[1.3]',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    transform: '[translate(-50%, -50%)]',
    transition: '[opacity 160ms ease-out, transform 200ms ease-out, color 200ms ease-out]',
    _motionReduce: { transition: '[none]' },
  });

  const withEyebrow = css.raw({
    color: 'text.default',
    textAlign: 'center',
    '&[data-state=""]': {
      opacity: '0',
      transform: '[translate(-50%, calc(-50% + 12px))]',
      pointerEvents: 'none',
    },
    '&[data-state="title"]': { transform: '[translate(-50%, calc(-50% + 8px))]' },
    _hover: { color: 'text.muted' },
  });

  const titleOnly = css.raw({
    color: 'text.default',
    textAlign: 'center',
    '&[data-state=""]': {
      opacity: '0',
      transform: '[translate(-50%, calc(-50% + 12px))]',
      pointerEvents: 'none',
    },
    '&[data-state="title"]': { transform: '[translate(-50%, -50%)]' },
    _hover: { color: 'text.muted' },
  });

  const pathname = $derived<string>(page.url.pathname);
  const activeTab = $derived<TabId | null>(
    pathname === discoveryHomePath
      ? 'home'
      : pathname === discoveryLatestPath
        ? 'latest'
        : pathname === discoveryTagsPath || pathname.startsWith('/t/')
          ? 'tags'
          : null,
  );
  const currentTag = $derived(pathname.startsWith('/t/') ? decodeURIComponent(pathname.slice(3)) : null);

  $effect(() => {
    query = initialQuery;
  });

  $effect(() => {
    chrome.hold = accountMenuOpen || searchOpen;
  });

  $effect(() => {
    void chrome.tick;
    scrolled = window.scrollY > 2;
  });

  const closeSearch = ({ restoreFocus = false } = {}) => {
    if (!searchOpen) return;
    searchOpen = false;
    if (restoreFocus) void tick().then(() => searchButton?.focus());
  };

  const toggleSearch = () => {
    if (chrome.searchInput) {
      chrome.searchInput.scrollIntoView({ block: 'center', behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
      chrome.searchInput.focus({ preventScroll: true });
      chrome.searchInput.select();
      return;
    }
    if (searchOpen) closeSearch();
    else searchOpen = true;
  };

  $effect(() => {
    if (!searchOpen) return;
    return pushEscapeHandler(() => {
      closeSearch({ restoreFocus: true });
      return true;
    });
  });

  $effect(() => {
    if (!searchOpen) return;
    const onpointerdown = (e: PointerEvent) => {
      const target = e.target as Element | null;
      if (!target || !root?.contains(target)) {
        closeSearch();
        return;
      }
      if (target.closest('[data-search-panel], [data-search-scrim], [data-search-toggle]')) return;
      closeSearch();
    };
    document.addEventListener('pointerdown', onpointerdown, { capture: true });
    return () => document.removeEventListener('pointerdown', onpointerdown, { capture: true });
  });

  afterNavigate(() => {
    closeSearch();
  });

  const onTabClick = (e: MouseEvent, id: TabId) => {
    if (id !== activeTab || pathname.startsWith('/t/')) return;
    e.preventDefault();
    window.scrollTo({ top: 0, behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
  };

  type Indicator = { x: number; width: number };

  let desktopNav = $state<HTMLElement>();
  let mobileNav = $state<HTMLElement>();
  let desktopIndicator = $state<Indicator>({ x: 0, width: 0 });
  let mobileIndicator = $state<Indicator>({ x: 0, width: 0 });
  let indicatorReady = $state(false);

  const measureNav = (nav: HTMLElement | undefined): Indicator => {
    const link = nav?.querySelector<HTMLElement>('a[aria-current="page"]');
    return link ? { x: link.offsetLeft, width: link.offsetWidth } : { x: 0, width: 0 };
  };

  const measureIndicators = () => {
    desktopIndicator = measureNav(desktopNav);
    mobileIndicator = measureNav(mobileNav);
  };

  $effect(() => {
    void activeTab;
    void tick().then(measureIndicators);
  });

  $effect(() => {
    if (!desktopNav && !mobileNav) {
      indicatorReady = false;
      return;
    }

    measureIndicators();

    const frame = requestAnimationFrame(() => {
      indicatorReady = true;
    });

    const observer = new ResizeObserver(measureIndicators);
    if (desktopNav) observer.observe(desktopNav);
    if (mobileNav) observer.observe(mobileNav);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      indicatorReady = false;
    };
  });

  const tabLink = css.raw({
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    fontSize: '15px',
    fontWeight: 'medium',
    color: 'text.hint',
    transition: 'colors',
    _hover: { color: 'text.muted' },
    _currentPage: { color: 'text.default' },
  });

  const staticIndicator = css.raw({
    '&:not([data-ready]) a[aria-current="page"]::after': {
      content: '""',
      position: 'absolute',
      left: '0',
      right: '0',
      bottom: '-1px',
      height: '2px',
      backgroundColor: 'text.default',
    },
  });

  const indicatorStyle = (ready: boolean) =>
    css(
      {
        position: 'absolute',
        left: '0',
        bottom: '-1px',
        height: '2px',
        backgroundColor: 'text.default',
        pointerEvents: 'none',
        willChange: 'transform',
      },
      ready && {
        transition: '[transform 200ms cubic-bezier(0.32, 0.72, 0, 1), width 200ms cubic-bezier(0.32, 0.72, 0, 1)]',
        _motionReduce: { transition: '[none]' },
      },
    );
</script>

<div
  bind:this={root}
  class={css({
    position: 'relative',
    transition: '[transform 160ms ease-out]',
    '&[data-hidden]': { transform: '[translateY(calc(-100% + 2px))]', transitionDuration: '[140ms]' },
    _motionReduce: { transition: '[none]' },
  })}
  data-hidden={hidden || undefined}
>
  <div
    class={css({
      position: 'relative',
      zIndex: '3',
      height: '52px',
      borderBottomWidth: '1px',
      borderColor: 'transparent',
      backgroundColor: 'surface.default',
      transition: '[border-color 150ms ease-out]',
      md: {
        '&[data-scrolled]:not([data-merged]), &[data-open]': { borderColor: 'border.default' },
      },
      '&[data-post][data-scrolled]:not([data-merged]), &[data-post][data-hidden]': { borderColor: 'border.default' },
      _motionReduce: { transition: '[none]' },
    })}
    data-hidden={hidden || undefined}
    data-merged={chrome.merged || undefined}
    data-open={searchOpen || undefined}
    data-post={isPost || undefined}
    data-scrolled={scrolled || undefined}
  >
    <div
      class={css(
        {
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          gap: '16px',
          height: 'full',
          marginX: 'auto',
          paddingX: { base: '20px', md: '40px' },
        },
        isPost ? { maxWidth: '1064px' } : { maxWidth: '1200px' },
      )}
    >
      {#if isPost && eyebrow}
        <a
          class={flex({
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: '0',
            size: '32px',
            marginLeft: '-14px',
            borderRadius: '8px',
            color: 'text.muted',
            transition: 'colors',
            _hover: { color: 'text.default' },
          })}
          aria-label={eyebrow.label}
          href={eyebrow.href}
          use:tooltip={{ message: eyebrow.label }}
        >
          <Icon icon={ChevronLeftIcon} size={16} />
        </a>
      {:else}
        <a class={flex({ alignItems: 'center', flexShrink: '0', height: '32px' })} aria-label="타이피" href={discoveryHomePath}>
          <ApexWordmark />
        </a>
      {/if}

      {#if !isPost}
        <nav
          bind:this={desktopNav}
          class={css(staticIndicator, {
            display: { base: 'none', md: 'flex' },
            position: 'relative',
            alignSelf: 'stretch',
            gap: '4px',
            marginLeft: '20px',
          })}
          aria-label="메뉴"
          data-ready={indicatorReady || undefined}
        >
          {#each TABS as tab (tab.id)}
            <a
              class={css(tabLink, { paddingX: '10px' })}
              aria-current={activeTab === tab.id ? 'page' : undefined}
              href={tab.href}
              onclick={(e) => onTabClick(e, tab.id)}
            >
              {tab.label}
            </a>
          {/each}
          {#if indicatorReady}
            <span
              style:width={`${activeTab ? desktopIndicator.width : 0}px`}
              style:transform={`translateX(${desktopIndicator.x}px)`}
              class={indicatorStyle(indicatorReady)}
              aria-hidden="true"
            ></span>
          {/if}
        </nav>
      {/if}

      {#if isPost && chrome.post}
        <div
          class={css({
            position: 'absolute',
            left: '1/2',
            top: '1/2',
            width: '[56%]',
            height: '32px',
            transform: '[translate(-50%, -50%)]',
          })}
        >
          {#if eyebrow}
            <a
              class={css(stackItem, {
                color: 'text.default',
                '&[data-state=""]': { opacity: '0', pointerEvents: 'none' },
                '&[data-state="title"]': {
                  transform: '[translate(-50%, calc(-50% - 8px)) scale(0.79)]',
                  fontWeight: 'medium',
                  color: 'text.hint',
                  _hover: { color: 'text.muted' },
                },
              })}
              data-state={slot}
              href={eyebrow.href}
            >
              {eyebrow.label}
            </a>
          {/if}
          <button
            class={css(stackItem, eyebrow ? withEyebrow : titleOnly)}
            data-state={slot}
            onclick={scrollToTop}
            title={chrome.post.title}
            type="button"
          >
            {chrome.post.title}
          </button>
        </div>
      {/if}

      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0', marginLeft: 'auto' })}>
        {#if !isPost}
          <button
            bind:this={searchButton}
            class={flex({
              alignItems: 'center',
              justifyContent: 'center',
              size: '32px',
              borderRadius: '8px',
              color: 'text.muted',
              transition: 'colors',
              _hover: { color: 'text.default' },
              _expanded: { color: 'text.default' },
            })}
            aria-controls="apex-search-panel"
            aria-expanded={searchOpen}
            aria-label="검색"
            data-search-toggle
            onclick={toggleSearch}
            type="button"
            use:tooltip={{ message: searchOpen ? null : '검색' }}
          >
            <Icon icon={searchOpen ? XIcon : SearchIcon} size={18} />
          </button>
        {/if}

        <AccountMenu {authorizeUrl} {onLogout} {user$key} bind:open={accountMenuOpen} />
      </div>
    </div>
  </div>

  {#if searchOpen && !isPost}
    <ApexSearchPanel id="apex-search-panel" {currentTag} onclose={() => closeSearch()} bind:query />
  {/if}

  {#if !isPost}
    <nav
      bind:this={mobileNav}
      class={css(staticIndicator, {
        display: { base: 'flex', md: 'none' },
        position: 'relative',
        zIndex: '3',
        gap: '4px',
        paddingX: '20px',
        borderBottomWidth: '1px',
        borderColor: 'border.hairline',
        backgroundColor: 'surface.default',
        transition: '[border-color 150ms ease-out]',
        '&[data-scrolled]': { borderColor: 'border.default' },
        '&[data-merged]': { borderColor: 'transparent' },
        _motionReduce: { transition: '[none]' },
      })}
      aria-label="메뉴"
      data-merged={chrome.merged || undefined}
      data-ready={indicatorReady || undefined}
      data-scrolled={scrolled || undefined}
    >
      {#each TABS as tab (tab.id)}
        <a
          class={css(tabLink, { height: '44px', paddingX: '12px', _first: { paddingLeft: '0' } })}
          aria-current={activeTab === tab.id ? 'page' : undefined}
          href={tab.href}
          onclick={(e) => onTabClick(e, tab.id)}
        >
          {tab.label}
        </a>
      {/each}
      {#if indicatorReady}
        <span
          style:width={`${activeTab ? mobileIndicator.width : 0}px`}
          style:transform={`translateX(${mobileIndicator.x}px)`}
          class={indicatorStyle(indicatorReady)}
          aria-hidden="true"
        ></span>
      {/if}
    </nav>
  {/if}

  {#if isPost}
    <span
      style:transform={`scaleX(${progress})`}
      class={css({
        position: 'absolute',
        left: '0',
        right: '0',
        bottom: '-1px',
        height: '2px',
        backgroundColor: 'text.default',
        transformOrigin: 'left',
        zIndex: '3',
      })}
      aria-hidden="true"
    ></span>
  {/if}
</div>
