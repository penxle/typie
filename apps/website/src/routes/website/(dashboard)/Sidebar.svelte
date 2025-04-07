<script lang="ts">
  import BellIcon from '~icons/lucide/bell';
  import FolderIcon from '~icons/lucide/folder';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import HomeIcon from '~icons/lucide/home';
  import PanelLeftCloseIcon from '~icons/lucide/panel-left-close';
  import SearchIcon from '~icons/lucide/search';
  import SettingsIcon from '~icons/lucide/settings';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
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

  const app = getAppContext();

  const createFolder = graphql(`
    mutation DashboardLayout_Sidebar_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);
</script>

<nav
  class={css({
    zIndex: '50',
    flexGrow: '0',
    flexShrink: '0',
    width: app.preference.current.sidebarExpanded ? '240px' : '0',
    transitionDuration: '200ms',
    transitionProperty: 'width',
    transitionTimingFunction: 'ease',
    pointerEvents: 'none',
    height: 'full',
  })}
  aria-hidden={!app.preference.current.sidebarExpanded && !app.state.sidebarPopoverVisible}
>
  <div
    class={css({
      position: 'absolute',
      top: '0',
      left: '0',
      bottom: '0',
      zIndex: '50',
      display: 'flex',
      flexDirection: 'column',
      width: '0',
      overflow: 'visible',
      pointerEvents: 'none',
    })}
  >
    <div
      class={css(
        {
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
          pointerEvents: 'auto',
          visibility: 'visible',
          paddingTop: '0',
          width: '240px',
          opacity: '100',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          transitionProperty: 'width, opacity, transform',
          backgroundColor: 'white',
          borderRightRadius: '10px',
          boxShadow: 'small',
        },
        app.preference.current.sidebarExpanded
          ? {
              height: 'full',
              transform: 'translateX(0) translateY(0)',
            }
          : {
              height: 'auto',
              transform: 'translateX(0) translateY(59px)',
            },

        !app.preference.current.sidebarExpanded &&
          !app.state.sidebarPopoverVisible && {
            opacity: '0',
            transform: 'translateX(-220px) translateY(59px)',
          },
      )}
      onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
      onpointerleave={() => (app.state.sidebarPopoverVisible = false)}
    >
      <div
        class={css({
          position: 'relative',
          top: '-15px',
          marginBottom: '-15px',
          height: '15px',
          width: '240px',
          backgroundColor: 'transparent',
        })}
        onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
      ></div>
      <div
        class={css(
          { position: 'absolute', inset: '0', zIndex: '[-1]', display: app.preference.current.sidebarExpanded ? 'none' : 'block' },
          !app.preference.current.sidebarExpanded &&
            app.state.sidebarPopoverVisible && {
              backgroundColor: 'white',
              borderRightRadius: '10px',
              boxShadow: 'small',
            },
        )}
      ></div>

      <div
        class={css({
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          height: 'full',
          maxHeight: app.preference.current.sidebarExpanded ? 'full' : '[calc(-118px + 100vh)]',
          overflowY: 'auto',
          padding: '10px',
        })}
        onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
        onpointerleave={() => (app.state.sidebarPopoverVisible = false)}
      >
        <div
          class={css({
            position: 'sticky',
            top: '0',
            backgroundColor: 'white',
            paddingBottom: '8px',
            zIndex: '1',
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

            <div class={flex({ align: 'center', gap: '4px' })}>
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
                    app.state.sidebarPopoverVisible = false;
                  }}
                  type="button"
                >
                  <Icon icon={PanelLeftCloseIcon} size={16} />
                </button>
              {/if}
            </div>
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
                새 글 쓰기
              </Button>
            </div>

            <ul
              class={css({
                display: 'flex',
                flexDirection: 'column',
                gap: '2px',
              })}
            >
              {#each [{ name: '홈', icon: HomeIcon }, { name: '검색', icon: SearchIcon }, { name: '알림', icon: BellIcon }, { name: '설정', icon: SettingsIcon }] as item (item.name)}
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
                    <Icon style={{ color: 'gray.500' }} icon={item.icon} size={16} />
                    {item.name}
                  </button>
                </li>
              {/each}
            </ul>
          </nav>
        </div>

        <div
          class={css({
            marginY: '8px',
            height: '1px',
            width: 'full',
            backgroundColor: 'gray.200',
          })}
        ></div>

        <div
          class={css({
            minHeight: '400px',
            paddingTop: '4px',
          })}
        >
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

          <PageList {entities} />
        </div>
      </div>
    </div>
  </div>
</nav>
