<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { MediaQuery } from 'svelte/reactivity';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import AccountMenu from './AccountMenu.svelte';
  import { currentSpaceSlug } from './apex/@[slug]/current-space-slug';
  import { seriesPath, spaceHomePath } from './apex/@[slug]/paths';
  import { getUsersiteChrome } from './chrome.svelte';
  import TypieMark from './TypieMark.svelte';
  import type { UsersiteHeader_spaceView$key, UsersiteHeader_user$key } from '$mearie';

  type Props = {
    spaceView$key: UsersiteHeader_spaceView$key;
    user$key: UsersiteHeader_user$key | null | undefined;
    authorizeUrl: string;
    stickyBottom: number;
    onLogout: () => void;
  };

  let { spaceView$key, user$key, authorizeUrl, stickyBottom, onLogout }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteHeader_spaceView on SpaceView {
        id
        name

        logo {
          id
          ...Img_image
        }
      }
    `),
    () => spaceView$key,
  );

  const chrome = getUsersiteChrome();
  const desktop = new MediaQuery('(min-width: 1024px)');

  let accountMenuOpen = $state(false);
  let scrolled = $state(false);
  let identityPast = $state(false);
  let slot = $state<'' | 'title'>('');
  let progress = $state(0);

  const slug = $derived(currentSpaceSlug());
  const isPost = $derived(chrome.post !== null);
  const eyebrow = $derived(
    chrome.post?.collection
      ? { label: chrome.post.collection.name, href: seriesPath(slug, chrome.post.collection.id) }
      : { label: space.data.name, href: spaceHomePath(slug) },
  );
  const hidden = $derived(isPost && chrome.retreat && !desktop.current);

  $effect(() => {
    chrome.hold = accountMenuOpen;
  });

  $effect(() => {
    void chrome.tick;
    void chrome.post;
    void chrome.titleEl;
    void chrome.identityEls;
    void stickyBottom;
    measure();
  });

  const measure = () => {
    const h = stickyBottom;
    scrolled = window.scrollY > 2;

    if (chrome.post) {
      const titleBottom = chrome.titleEl?.getBoundingClientRect().bottom;
      slot = titleBottom !== undefined && titleBottom <= h ? 'title' : '';

      const max = document.documentElement.scrollHeight - window.innerHeight;
      progress = max > 0 ? Math.min(1, Math.max(0, window.scrollY / max)) : 0;
      identityPast = false;
    } else {
      slot = '';
      progress = 0;
      const visible = chrome.identityEls.filter((el) => el.offsetParent !== null);
      identityPast = visible.length > 0 && visible.every((el) => el.getBoundingClientRect().bottom <= h);
    }
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
</script>

<div
  class={css({
    position: 'relative',
    height: '52px',
    borderBottomWidth: '1px',
    borderColor: 'transparent',
    backgroundColor: 'surface.default',
    transition: '[transform 160ms ease-out, border-color 150ms ease-out]',
    '&[data-scrolled]:not([data-merged])': { borderColor: 'border.default' },
    '&[data-hidden]': { transform: '[translateY(calc(-100% + 2px))]', transitionDuration: '[140ms]', borderColor: 'border.default' },
    _motionReduce: { transition: '[none]' },
  })}
  data-hidden={hidden || undefined}
  data-merged={chrome.merged || undefined}
  data-scrolled={scrolled || undefined}
>
  <div
    class={flex({
      position: 'relative',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '16px',
      height: 'full',
      maxWidth: '1064px',
      marginX: 'auto',
      paddingX: { base: '20px', md: '40px' },
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '8px', flex: '1', minWidth: '0' })}>
      {#if isPost}
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
          aria-label={space.data.name}
          href={spaceHomePath(slug)}
          use:tooltip={{ message: space.data.name }}
        >
          <Icon icon={ChevronLeftIcon} size={16} />
        </a>
      {:else}
        <a
          class={flex({
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: '0',
            size: '32px',
            marginLeft: '-4px',
            borderRadius: '8px',
            color: 'text.default',
          })}
          aria-label="타이피"
          href="/"
          use:tooltip={{ message: '타이피' }}
        >
          <TypieMark size={24} />
        </a>

        <div
          class={flex({
            alignItems: 'center',
            gap: '8px',
            minWidth: '0',
            transition: '[opacity 160ms ease-out]',
            '&:not([data-past])': { opacity: '0', pointerEvents: 'none' },
            _motionReduce: { transition: '[none]' },
          })}
          data-past={identityPast || undefined}
        >
          <span class={css({ flexShrink: '0', width: '1px', height: '16px', backgroundColor: 'border.default' })} aria-hidden="true"></span>
          <a
            class={flex({
              alignItems: 'center',
              gap: '8px',
              minWidth: '0',
              height: '32px',
              paddingLeft: '4px',
              paddingRight: '8px',
              borderRadius: '8px',
              fontSize: '14px',
              fontWeight: 'semibold',
              letterSpacing: '-0.01em',
              color: 'text.default',
              transition: 'colors',
              _hover: { color: 'text.muted' },
            })}
            href={spaceHomePath(slug)}
          >
            <Img
              style={css.raw({
                flexShrink: '0',
                size: '24px',
                borderRadius: '6px',
                objectFit: 'cover',
                boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
              })}
              alt={`${space.data.name} 로고`}
              image$key={space.data.logo}
              size={48}
            />
            <span class={css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>{space.data.name}</span>
          </a>
        </div>
      {/if}
    </div>

    {#if isPost && chrome.post}
      <div
        class={css({ position: 'absolute', left: '1/2', top: '1/2', width: '[56%]', height: '32px', transform: '[translate(-50%, -50%)]' })}
      >
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
        <button
          class={css(stackItem, {
            color: 'text.default',
            textAlign: 'center',
            '&[data-state=""]': {
              opacity: '0',
              transform: '[translate(-50%, calc(-50% + 12px))]',
              pointerEvents: 'none',
            },
            '&[data-state="title"]': { transform: '[translate(-50%, calc(-50% + 8px))]' },
            _hover: { color: 'text.muted' },
          })}
          data-state={slot}
          onclick={scrollToTop}
          title={chrome.post.title}
          type="button"
        >
          {chrome.post.title}
        </button>
      </div>
    {/if}

    <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
      <AccountMenu {authorizeUrl} {onLogout} {user$key} bind:open={accountMenuOpen} />
    </div>
  </div>

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
      })}
      aria-hidden="true"
    ></span>
  {/if}
</div>
