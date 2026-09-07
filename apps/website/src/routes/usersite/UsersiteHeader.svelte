<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { MediaQuery } from 'svelte/reactivity';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import HouseIcon from '~icons/lucide/house';
  import LogOutIcon from '~icons/lucide/log-out';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { getUsersiteChrome } from './chrome.svelte';
  import ThemeSegment from './ThemeSegment.svelte';
  import TypieMark from './TypieMark.svelte';
  import { seriesPath } from './wildcard/paths';
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

  const user = createFragment(
    graphql(`
      fragment UsersiteHeader_user on User {
        id
        name

        avatar {
          id
          ...Img_image
        }
      }
    `),
    () => user$key,
  );

  const chrome = getUsersiteChrome();
  const desktop = new MediaQuery('(min-width: 1024px)');

  let accountMenuOpen = $state(false);
  let scrolled = $state(false);
  let identityPast = $state(false);
  let slot = $state<'' | 'title'>('');
  let progress = $state(0);

  const isPost = $derived(chrome.post !== null);
  const eyebrow = $derived(
    chrome.post?.collection
      ? { label: chrome.post.collection.name, href: seriesPath(chrome.post.collection.id) }
      : { label: space.data.name, href: '/' },
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

  const menuRow = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    paddingX: '8px',
    paddingY: '6px',
    borderRadius: '6px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.default',
    textAlign: 'left',
    outlineWidth: '0',
    transition: 'common',
    cursor: 'pointer',
    _hover: { backgroundColor: 'surface.hover' },
    _focus: { backgroundColor: 'surface.hover' },
  });

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
          href="/"
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
          href={env.PUBLIC_WEBSITE_URL}
          rel="noopener noreferrer"
          target="_blank"
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
            href="/"
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
      {#if user.data}
        <Menu
          style={css.raw({
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            size: '32px',
            padding: '2px',
            marginRight: '-2px',
            borderRadius: 'full',
            transition: '[opacity 150ms ease-out]',
            _hover: { opacity: '[0.8]' },
            _expanded: { opacity: '[0.8]' },
          })}
          listStyle={css.raw({
            gap: '0',
            minWidth: '200px',
            padding: '4px',
            borderWidth: '1px',
            borderColor: 'border.default',
            boxShadow: 'md',
          })}
          offset={6}
          placement="bottom-end"
          bind:open={accountMenuOpen}
        >
          {#snippet button()}
            {#if user.data?.avatar}
              <Img
                style={css.raw({ size: '28px', borderRadius: 'full', boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]' })}
                alt={`${user.data.name}의 아바타`}
                image$key={user.data.avatar}
                size={64}
              />
            {:else}
              <div class={css({ size: '28px', borderRadius: 'full', backgroundColor: 'accent.subtle' })}></div>
            {/if}
          {/snippet}

          <div class={flex({ alignItems: 'center', gap: '8px', paddingX: '8px', paddingY: '6px' })} role="none">
            {#if user.data.avatar}
              <Img style={css.raw({ flexShrink: '0', size: '24px', borderRadius: 'full' })} alt="" image$key={user.data.avatar} size={32} />
            {:else}
              <div class={css({ flexShrink: '0', size: '24px', borderRadius: 'full', backgroundColor: 'accent.subtle' })}></div>
            {/if}
            <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', truncate: true })}>{user.data.name}</span>
          </div>

          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

          <a class={css(menuRow)} href={env.PUBLIC_WEBSITE_URL} role="menuitem" tabindex="-1">
            <Icon style={css.raw({ flexShrink: '0', color: 'text.default' })} icon={HouseIcon} size={14} />
            <span>내 홈으로</span>
          </a>

          <ThemeSegment via="header" />

          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

          <button
            class={css(menuRow, { color: 'danger.default' })}
            onclick={() => {
              accountMenuOpen = false;
              onLogout();
            }}
            role="menuitem"
            tabindex="-1"
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0' })} icon={LogOutIcon} size={14} />
            <span>로그아웃</span>
          </button>
        </Menu>
      {:else}
        <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
      {/if}
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
