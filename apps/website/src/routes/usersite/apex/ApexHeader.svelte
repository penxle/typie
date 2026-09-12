<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, TextInput } from '@typie/ui/components';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import SearchIcon from '~icons/lucide/search';
  import XIcon from '~icons/lucide/x';
  import { afterNavigate } from '$app/navigation';
  import AccountMenu from '../AccountMenu.svelte';
  import { getUsersiteChrome } from '../chrome.svelte';
  import TypieMark from '../TypieMark.svelte';
  import type { UsersiteHeader_user$key } from '$mearie';

  type Props = {
    user$key: UsersiteHeader_user$key | null | undefined;
    authorizeUrl: string;
    onLogout: () => void;
    initialQuery?: string;
  };

  let { user$key, authorizeUrl, onLogout, initialQuery = '' }: Props = $props();

  const chrome = getUsersiteChrome();

  let accountMenuOpen = $state(false);
  let scrolled = $state(false);
  let mobileSearchOpen = $state(false);
  let mobileInput = $state<HTMLInputElement>();
  let query = $state(initialQuery);

  $effect(() => {
    query = initialQuery;
  });

  $effect(() => {
    chrome.hold = accountMenuOpen || mobileSearchOpen;
  });

  $effect(() => {
    void chrome.tick;
    scrolled = window.scrollY > 2;
  });

  const openMobileSearch = async () => {
    mobileSearchOpen = true;
    await tick();
    mobileInput?.focus();
  };

  const closeMobileSearch = () => {
    mobileSearchOpen = false;
  };

  $effect(() => {
    if (!mobileSearchOpen) return;
    return pushEscapeHandler(() => {
      closeMobileSearch();
      return true;
    });
  });

  afterNavigate(() => {
    closeMobileSearch();
  });
</script>

<div>
  <div
    class={css({
      position: 'relative',
      height: '52px',
      borderBottomWidth: '1px',
      borderColor: 'transparent',
      backgroundColor: 'surface.default',
      transition: '[border-color 150ms ease-out]',
      '&[data-scrolled]:not([data-merged])': { borderColor: 'border.default' },
      _motionReduce: { transition: '[none]' },
    })}
    data-merged={chrome.merged || undefined}
    data-scrolled={scrolled || undefined}
  >
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '16px',
        height: 'full',
        maxWidth: '1064px',
        marginX: 'auto',
        paddingX: { base: '20px', md: '40px' },
      })}
    >
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

      <form class={css({ display: { base: 'none', md: 'block' }, flex: '1', maxWidth: '480px' })} action="/search" method="GET">
        <TextInput name="q" leftIcon={SearchIcon} placeholder="글, 스페이스, 태그 검색" size="sm" type="search" bind:value={query} />
      </form>

      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <button
          class={flex({
            display: { base: 'inline-flex', md: 'none' },
            alignItems: 'center',
            justifyContent: 'center',
            size: '32px',
            borderRadius: '8px',
            color: 'text.muted',
            transition: 'colors',
            _hover: { color: 'text.default' },
            _expanded: { color: 'text.default' },
          })}
          aria-controls="apex-mobile-search"
          aria-expanded={mobileSearchOpen}
          aria-label="검색"
          onclick={() => (mobileSearchOpen ? closeMobileSearch() : void openMobileSearch())}
          type="button"
          use:tooltip={{ message: '검색' }}
        >
          <Icon icon={mobileSearchOpen ? XIcon : SearchIcon} size={18} />
        </button>

        <AccountMenu {authorizeUrl} {onLogout} {user$key} bind:open={accountMenuOpen} />
      </div>
    </div>
  </div>

  {#if mobileSearchOpen}
    <div
      id="apex-mobile-search"
      class={css({ display: { base: 'block', md: 'none' }, paddingX: '20px', paddingBottom: '12px', backgroundColor: 'surface.default' })}
    >
      <form action="/search" method="GET">
        <TextInput
          name="q"
          leftIcon={SearchIcon}
          placeholder="글, 스페이스, 태그 검색"
          size="sm"
          type="search"
          bind:element={mobileInput}
          bind:value={query}
        />
      </form>
    </div>
  {/if}
</div>
