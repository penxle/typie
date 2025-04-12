<script lang="ts">
  import BellIcon from '~icons/lucide/bell';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import HomeIcon from '~icons/lucide/home';
  import PanelLeftCloseIcon from '~icons/lucide/panel-left-close';
  import PencilLineIcon from '~icons/lucide/pencil-line';
  import SearchIcon from '~icons/lucide/search';
  import SettingsIcon from '~icons/lucide/settings';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
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
  style:--threshold-width="200px"
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
  aria-hidden={!app.preference.current.sidebarExpanded && !app.state.sidebarTriggered}
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
    class={flex({
      flexDirection: 'column',
      borderRightRadius: '10px',
      padding: '10px',
      width: 'var(--expanded-width)',
      height: 'full',
      backgroundColor: 'white',
      boxShadow: 'small',
      transitionProperty: 'transform',
      transitionDuration: '200ms',
      transitionTimingFunction: 'ease',
      transform:
        app.preference.current.sidebarExpanded || app.state.sidebarTriggered
          ? 'translateX(0)'
          : `translateX(calc(var(--expanded-width) * -1))`,
      willChange: 'transform',
      pointerEvents: 'auto',
    })}
  >
    <div
      class={flex({
        align: 'center',
        justify: 'space-between',
        paddingY: '6px',
      })}
    >
      <Logo
        class={css({
          height: '24px',
          flex: 'none',
        })}
      />

      {#if app.preference.current.sidebarExpanded}
        <button
          class={css({
            padding: '4px',
            borderRadius: '6px',
            color: 'gray.500',
            _hover: {
              backgroundColor: 'gray.100',
              color: 'gray.700',
            },
          })}
          onclick={() => {
            app.preference.current.sidebarExpanded = false;
            app.state.sidebarTriggered = false;
          }}
          type="button"
        >
          <Icon icon={PanelLeftCloseIcon} size={16} />
        </button>
      {/if}
    </div>

    <nav
      class={css({
        marginTop: '12px',
      })}
    >
      <div class={css({ marginBottom: '12px' })}>
        <Button
          style={css.raw({ width: 'full' })}
          onclick={async () => {
            const resp = await createPost({
              siteId: $user.sites[0].id,
            });

            await goto(`/${resp.entity.slug}`);
          }}
          variant="primary"
        >
          <div class={center({ gap: '6px' })}>
            <Icon icon={PencilLineIcon} size={16} />
            새 글 쓰기
          </div>
        </Button>
      </div>

      <ul
        class={css({
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
        })}
      >
        <li>
          <a
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              width: 'full',
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.600',
              backgroundColor: 'transparent',
              _hover: {
                backgroundColor: 'gray.100',
              },
            })}
            href="/home"
          >
            <Icon style={{ color: 'gray.500' }} icon={HomeIcon} size={16} />
            홈
          </a>
        </li>
        <li>
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              width: 'full',
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.600',
              backgroundColor: 'transparent',
              _hover: {
                backgroundColor: 'gray.100',
              },
            })}
            type="button"
          >
            <Icon style={{ color: 'gray.500' }} icon={SearchIcon} size={16} />
            검색
          </button>
        </li>
        <li>
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              width: 'full',
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.600',
              backgroundColor: 'transparent',
              _hover: {
                backgroundColor: 'gray.100',
              },
            })}
            type="button"
          >
            <Icon style={{ color: 'gray.500' }} icon={BellIcon} size={16} />
            알림
          </button>
        </li>
        <li>
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              width: 'full',
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.600',
              backgroundColor: 'transparent',
              _hover: {
                backgroundColor: 'gray.100',
              },
            })}
            type="button"
          >
            <Icon style={{ color: 'gray.500' }} icon={SettingsIcon} size={16} />
            설정
          </button>
        </li>
      </ul>
    </nav>

    <div class={css({ paddingTop: '4px' })}>
      <div class={flex({ align: 'center', justify: 'space-between' })}>
        <div
          class={css({
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '8px',
            paddingY: '6px',
            marginBottom: '4px',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'gray.700',
          })}
        >
          <Icon style={{ color: 'gray.500' }} icon={FolderIcon} size={14} />
          <span>보관함</span>
        </div>

        <button
          class={css({
            padding: '4px',
            borderRadius: '6px',
            color: 'gray.500',
            _hover: {
              backgroundColor: 'gray.100',
              color: 'gray.700',
            },
          })}
          onclick={async () => {
            await createFolder({ siteId: $user.sites[0].id, name: '새 폴더' });
          }}
          type="button"
        >
          <Icon icon={FolderPlusIcon} size={14} />
        </button>
      </div>
    </div>

    <div class={css({ flexGrow: '1', overflow: 'scroll' })}>
      <PageList {entities} siteId={$user.sites[0].id} />
    </div>
  </div>
</aside>
