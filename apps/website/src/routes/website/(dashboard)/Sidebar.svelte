<script lang="ts">
  import { sineInOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import BellIcon from '~icons/lucide/bell';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import HomeIcon from '~icons/lucide/home';
  import PanelLeftCloseIcon from '~icons/lucide/panel-left-close';
  import SearchIcon from '~icons/lucide/search';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
  import UserMenu from './UserMenu.svelte';
  import type { DashboardLayout_Sidebar_user } from '$graphql';
  import type { Entity } from './types';

  type Props = {
    $user: DashboardLayout_Sidebar_user;
    entities: Entity[];
  };

  let { $user: _user, entities }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id

        sites {
          id
        }

        ...DashboardLayout_UserMenu_user
      }
    `),
  );

  const app = getAppContext();

  const createPost = graphql(`
    mutation DashboardLayout_Sidebar_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const createFolder = graphql(`
    mutation DashboardLayout_Sidebar_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);
</script>

<aside
  style:--expanded-width="240px"
  style:--threshold-width="48px"
  style:--y-offset="60px"
  class={css({
    position: 'relative',
    flexShrink: '0',
    width: app.preference.current.sidebarExpanded ? 'var(--expanded-width)' : '0',
    height: 'full',
    paddingY: app.preference.current.sidebarExpanded ? '0' : 'var(--y-offset)',
    transitionProperty: 'width, padding',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    willChange: 'width, padding',
    pointerEvents: 'none',
    zIndex: '50',
  })}
>
  {#if !app.preference.current.sidebarExpanded}
    {#if app.state.sidebarTriggered}
      <div
        class={css({
          position: 'absolute',
          left: 'var(--expanded-width)',
          insetY: '0',
          width: '[calc(100vw - var(--expanded-width))]',
          pointerEvents: 'auto',
        })}
        onpointerenter={() => (app.state.sidebarTriggered = false)}
      ></div>
    {:else}
      <div
        class={css({
          position: 'absolute',
          left: '0',
          insetY: '0',
          width: 'var(--threshold-width)',
          pointerEvents: 'auto',
        })}
        onpointerenter={() => (app.state.sidebarTriggered = true)}
      ></div>
    {/if}
  {/if}

  <div
    class={css(
      {
        display: 'flex',
        flexDirection: 'column',
        width: 'var(--expanded-width)',
        height: 'full',
        maxHeight: 'full',
        backgroundColor: 'gray.50',
        transitionProperty: 'background-color, border-width, border-radius, box-shadow, transform',
        transitionDuration: '200ms',
        transitionTimingFunction: 'ease',
        transform:
          app.preference.current.sidebarExpanded || app.state.sidebarTriggered
            ? 'translateX(0)'
            : `translateX(calc(var(--expanded-width) * -1))`,
        willChange: 'background-color, border-width, border-radius, box-shadow, transform',
        pointerEvents: 'auto',
      },
      !app.preference.current.sidebarExpanded && {
        borderYWidth: '1px',
        borderRightWidth: '1px',
        borderRightRadius: '12px',
        backgroundColor: 'white',
        boxShadow: '[2px 0 8px -1px rgba(0, 0, 0, 0.1)]',
      },
    )}
    inert={!app.preference.current.sidebarExpanded && !app.state.sidebarTriggered}
  >
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginTop: '16px', marginX: '16px' })}>
      <Logo class={css({ flexShrink: '0', height: '18px', color: 'brand.500' })} />

      {#if app.preference.current.sidebarExpanded}
        <button
          class={center({
            borderRadius: '6px',
            size: '24px',
            color: 'gray.500',
            _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
          })}
          onclick={() => {
            app.preference.current.sidebarExpanded = false;
            app.state.sidebarTriggered = true;
          }}
          type="button"
          transition:fade={{ duration: 100, easing: sineInOut }}
        >
          <Icon icon={PanelLeftCloseIcon} size={16} />
        </button>
      {/if}
    </div>

    <button
      class={css({ position: 'relative', marginTop: '16px', marginX: '16px', cursor: 'text', userSelect: 'none' })}
      onclick={() => (app.state.commandPaletteOpen = true)}
      type="button"
    >
      <input
        class={css({
          width: 'full',
          borderWidth: '1px',
          borderRadius: '6px',
          paddingLeft: '32px',
          paddingRight: '32px',
          paddingY: '6px',
          fontSize: '14px',
          backgroundColor: 'gray.100',
          pointerEvents: 'none',
          // boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.05)]',
        })}
        placeholder="검색"
        type="text"
      />

      <div class={center({ position: 'absolute', left: '8px', top: '1/2', translate: 'auto', translateY: '-1/2', pointerEvents: 'none' })}>
        <Icon icon={SearchIcon} size={16} />
      </div>

      <div class={center({ position: 'absolute', right: '8px', top: '1/2', translate: 'auto', translateY: '-1/2', pointerEvents: 'none' })}>
        <kbd
          class={center({
            gap: '2px',
            borderRadius: '4px',
            paddingX: '6px',
            paddingY: '2px',
            fontFamily: 'mono',
            fontSize: '12px',
            fontWeight: 'medium',
            color: 'gray.400',
            backgroundColor: 'gray.200',
          })}
        >
          <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
          {#if !navigator.platform.includes('Mac')}
            <span>+</span>
          {/if}
          <span>K</span>
        </kbd>
      </div>
    </button>

    <ul class={css({ display: 'flex', flexDirection: 'column', gap: '2px', marginTop: '8px', marginX: '16px' })}>
      <li>
        <a
          class={cx(
            'group',
            flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              width: 'full',
              _hover: { backgroundColor: 'gray.100' },
            }),
          )}
          href="/home"
        >
          <Icon style={{ color: 'gray.500', _groupHover: { color: 'gray.800' } }} icon={HomeIcon} size={16} />
          <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.700', _groupHover: { color: 'gray.950' } })}>홈</span>
        </a>
      </li>

      <li>
        <button
          class={cx(
            'group',
            flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              width: 'full',
              _hover: { backgroundColor: 'gray.100' },
            }),
          )}
          type="button"
        >
          <Icon style={{ color: 'gray.500', _groupHover: { color: 'gray.800' } }} icon={BellIcon} size={16} />
          <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.700', _groupHover: { color: 'gray.950' } })}>알림</span>
        </button>
      </li>
    </ul>

    <div class={flex({ alignItems: 'center', gap: '4px', marginTop: '8px', marginX: '16px' })}>
      <div class={css({ flexGrow: '1', fontSize: '13px', color: 'gray.500' })}>보관함</div>

      <button
        class={center({
          borderRadius: '6px',
          size: '24px',
          color: 'gray.500',
          _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
        })}
        onclick={async () => {
          await createFolder({ siteId: $user.sites[0].id, name: '새 폴더' });
        }}
        type="button"
      >
        <Icon icon={FolderPlusIcon} size={14} />
      </button>

      <button
        class={center({
          borderRadius: '6px',
          size: '24px',
          color: 'gray.500',
          _hover: { color: 'gray.700', backgroundColor: 'gray.100' },
        })}
        onclick={async () => {
          const resp = await createPost({
            siteId: $user.sites[0].id,
          });

          await goto(`/${resp.entity.slug}`);
        }}
        type="button"
      >
        <Icon icon={SquarePenIcon} size={14} />
      </button>
    </div>

    <div class={css({ position: 'relative', flexGrow: '1', overflow: 'hidden' })}>
      <div class={css({ paddingX: '16px', overflow: 'auto', height: 'full' })}>
        <PageList {entities} siteId={$user.sites[0].id} />
      </div>

      <div
        class={css({
          position: 'absolute',
          insetX: '0',
          top: '0',
          height: '16px',
          backgroundGradient: 'to-b',
          gradientFrom: app.preference.current.sidebarExpanded ? 'gray.50' : 'white',
          gradientTo: 'transparent',
          pointerEvents: 'none',
        })}
      ></div>

      <div
        class={css({
          position: 'absolute',
          insetX: '0',
          bottom: '0',
          height: '16px',
          backgroundGradient: 'to-t',
          gradientFrom: app.preference.current.sidebarExpanded ? 'gray.50' : 'white',
          gradientTo: 'transparent',
          pointerEvents: 'none',
        })}
      ></div>
    </div>

    <UserMenu {$user} />
  </div>
</aside>
